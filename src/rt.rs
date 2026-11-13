#![allow(unsafe_op_in_unsafe_fn)]
use std::alloc::{Layout, alloc_zeroed, dealloc, realloc};
use std::collections::HashMap;
use std::ffi::{CStr, c_void};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Once, OnceLock};

use crate::trace::{
    TRACE_CORO_CLOSE, TRACE_CORO_DEAD, TRACE_CORO_RESUME, TRACE_CORO_SPAWN, TRACE_CORO_YIELD,
    TRACE_CORO_YIELD_DROPPED, TRACE_FAIL_ANY_LEN, TRACE_FAIL_ASSERT, TRACE_FAIL_CORO_CREATE,
    TRACE_FAIL_CORO_YIELD_RESERVED, TRACE_FAIL_DIV_ZERO, TRACE_FAIL_ERROR_RAISED,
    TRACE_FAIL_FFI_LOAD, TRACE_FAIL_FFI_SYM, TRACE_FAIL_LEAK_DETECTED, TRACE_FAIL_NULL_ROW_STORE,
    TRACE_FAIL_TONUMBER, TRACE_RT_ALLOC, TRACE_RT_FFI_LOAD, TRACE_RT_FREE, TRACE_RT_FREE_CHILD,
    TRACE_RT_RESERVE_NULL,
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableMode {
    Dense = 0,
    Sparse = 1,
}

#[repr(C)]
pub struct AlmTable {
    pub data: *mut u8,
    pub len: i64,
    pub reserve: usize,
    pub esize: usize,
    pub mode: TableMode,
    pub contains_tables: u8,
    pub elem_kind: u8,
    pub sparse_map: *mut HashMap<i64, u128>,
    pub border: i64,
}

/// Raw byte-discriminant mirror of `AlmTable` for corruption/double-free
/// audits so reading a freed or corrupted header never materializes an
/// invalid `TableMode` enum discriminant (which would be immediate UB).
#[repr(C)]
#[derive(Clone, Copy)]
struct RawAlmTable {
    data: *mut u8,
    len: i64,
    reserve: usize,
    esize: usize,
    mode: u8,
    contains_tables: u8,
    elem_kind: u8,
    sparse_map: *mut HashMap<i64, u128>,
    border: i64,
}

const _: () = {
    use std::mem::{align_of, offset_of, size_of};
    assert!(
        size_of::<AlmTable>() == 56,
        "AlmTable must stay the 56-byte ABI"
    );
    assert!(size_of::<RawAlmTable>() == size_of::<AlmTable>());
    assert!(align_of::<RawAlmTable>() == align_of::<AlmTable>());

    macro_rules! assert_abi_offsets {
        ($( $field:ident == $off:expr ),* $(,)?) => {
            $(
                assert!(offset_of!(AlmTable, $field) == $off);
                assert!(offset_of!(RawAlmTable, $field) == $off);
            )*
        };
    }

    assert_abi_offsets! {
        data == 0,
        len == 8,
        reserve == 16,
        esize == 24,
        mode == 32,
        contains_tables == 33,
        elem_kind == 34,
        sparse_map == 40,
        border == 48,
    }
};

static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);
static TOTAL_COUNT: AtomicUsize = AtomicUsize::new(0);

const EXEC_UNREGISTERED: i32 = -2;
static EXEC_ID: OnceLock<i32> = OnceLock::new();
static IS_EXE: AtomicBool = AtomicBool::new(false);
static LOCAL_LEAK_HOOK: Once = Once::new();

#[cfg_attr(miri, allow(dead_code))]
unsafe extern "C" {
    fn vx_trace_poke_exec(exec_id: i32, slot: u8);
    fn vx_str_intern(s: *const u8, len: usize) -> *const u8;
    fn vx_str_register(strtab: *mut *const u8, n: usize);
    fn vx_exec_register(name: *const i8, flags: u32) -> i32;
    fn vx_exec_census_set(id: i32, live: i64, total: i64);
    fn vx_exec_state_set(id: i32, state: u32);
    fn vx_exec_count() -> i32;
    fn vx_coro_create(lib: *mut c_void) -> i32;
    fn vx_coro_send(id: i32, a: u64, b: u64, c: u64);
    fn vx_coro_start(id: i32) -> i32;
    fn vx_coro_pull(id: i32, done: *mut i32) -> i64;
    fn vx_coro_yield(value: u64) -> i64;
    fn vx_coro_status(id: i32) -> u32;
    fn vx_coro_join(id: i32);
    fn dladdr(addr: *const c_void, info: *mut DlInfo) -> i32;
}

#[repr(C)]
#[cfg_attr(miri, allow(dead_code))]
struct DlInfo {
    dli_fname: *const i8,
    dli_fbase: *mut c_void,
    dli_sname: *const i8,
    dli_saddr: *mut c_void,
}

#[cfg_attr(miri, allow(dead_code))]
static MODULE_MARK: u8 = 0;

#[inline]
fn current_exec_id() -> i32 {
    EXEC_ID.get().copied().unwrap_or(EXEC_UNREGISTERED)
}

fn exec_id() -> i32 {
    *EXEC_ID.get_or_init(|| unsafe { register_now() })
}

// Under Miri the vx sidecar and dladdr are foreign functions the
// interpreter cannot execute (they are observability and name
// introspection, not memory law), so the four touchpoints register as
// a fixed exec id and drop the trace pokes. The memory laws the
// interpreter is here to check — set_core, free_tbl, the strkey
// reserved range — are pure Rust and run in full.
#[cfg(miri)]
unsafe fn register_now() -> i32 {
    0
}

#[cfg(not(miri))]
unsafe fn register_now() -> i32 {
    let mut info: DlInfo = std::mem::zeroed();
    let name: Vec<u8> = if unsafe { dladdr(ptr::addr_of!(MODULE_MARK).cast::<c_void>(), &mut info) }
        != 0
        && !info.dli_fname.is_null()
    {
        let full = unsafe { cstr_bytes(info.dli_fname.cast()) };
        match full.iter().rposition(|&b| b == b'/') {
            Some(at) => full[at + 1..].to_vec(),
            None => full.to_vec(),
        }
    } else {
        b"alm".to_vec()
    };
    let mut nul = name;
    nul.push(0);
    let flags: u32 = if IS_EXE.load(Ordering::Relaxed) {
        0x1
    } else {
        0x2
    };
    unsafe { vx_exec_register(nul.as_ptr().cast(), flags) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_trace_set(slot: u8) {
    #[cfg(miri)]
    {
        let _ = slot;
    }
    #[cfg(not(miri))]
    unsafe {
        vx_trace_poke_exec(current_exec_id(), slot)
    };
}

/// Centralized cold trace-and-abort handler to keep hot paths compact.
#[cold]
#[inline(never)]
fn fatal_abort(trace_slot: Option<u8>, msg: std::fmt::Arguments<'_>) -> ! {
    if let Some(slot) = trace_slot {
        unsafe { alm_trace_set(slot) };
    }
    eprintln!("{msg}");
    std::process::abort();
}

macro_rules! alm_fatal {
    (trace: $slot:expr, $($arg:tt)*) => {
        fatal_abort(Some($slot), format_args!($($arg)*))
    };
    ($($arg:tt)*) => {
        fatal_abort(None, format_args!($($arg)*))
    };
}

unsafe fn mirror_census(id: i32) {
    #[cfg(miri)]
    {
        let _ = id;
    }
    #[cfg(not(miri))]
    if id >= 0 {
        unsafe {
            vx_exec_census_set(
                id,
                ALLOC_COUNT.load(Ordering::Relaxed) as i64,
                TOTAL_COUNT.load(Ordering::Relaxed) as i64,
            )
        };
    }
}

unsafe extern "C" fn alm_trace_leak_check() {
    if ALLOC_COUNT.load(Ordering::Relaxed) > 0 {
        unsafe { alm_trace_set(TRACE_FAIL_LEAK_DETECTED) };
        eprintln!("ALM_TRACE: slot 90 — leak at exit (sys_alloc_count() > 0)");
    }
}

/// The one checked size law for span arithmetic: `len` cells of
/// `esize` bytes each, as usize — None when the length or the product
/// does not fit. elem_layout and span_grow abort through it (a span
/// the process cannot address is a fatal allocation); the free-time
/// audit reads the same law as its validity predicate.
fn bytes_of(len: i64, esize: usize) -> Option<usize> {
    usize::try_from(len).ok().and_then(|l| l.checked_mul(esize))
}

fn elem_layout(len: i64, esize: usize) -> Layout {
    let bytes = bytes_of(len, esize).unwrap_or_else(|| abort_alloc(usize::MAX));
    Layout::from_size_align(bytes, span_align(esize)).unwrap_or_else(|_| abort_alloc(bytes))
}

const fn span_align(esize: usize) -> usize {
    if esize == u128::ESIZE {
        u128::ALIGN
    } else {
        u64::ALIGN
    }
}

#[cold]
#[inline(never)]
fn abort_alloc(bytes: usize) -> ! {
    alm_fatal!("alm: table allocation of {bytes} bytes failed")
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_tbl_new(esize: usize, flags: u8) -> *mut AlmTable {
    let contains_tables = (flags & 0x80) >> 7;
    let elem_kind = (flags >> 1) & 0x07;
    let is_sparse = (flags & 0x01) != 0;

    let hdr = Box::into_raw(Box::new(AlmTable {
        data: std::ptr::null_mut(),
        len: 0,
        reserve: 0,
        esize,
        mode: if is_sparse {
            TableMode::Sparse
        } else {
            TableMode::Dense
        },
        contains_tables,
        elem_kind,
        sparse_map: if is_sparse {
            Box::into_raw(Box::new(HashMap::new()))
        } else {
            std::ptr::null_mut()
        },
        border: 0,
    }));
    // The mint exposes the header's provenance: every lane word the
    // runtime and the IR exchange carries this address as an integer,
    // and the reconstitution seam (ptr::with_exposed_provenance in
    // set_core's refuse arm, reclaim_displaced_child, free_tbl's child
    // walk) can only recover provenance that was exposed here.
    hdr.expose_provenance();
    ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
    TOTAL_COUNT.fetch_add(1, Ordering::Relaxed);
    let id = exec_id();
    unsafe { mirror_census(id) };
    unsafe { alm_trace_set(TRACE_RT_ALLOC) };
    if id == -1 {
        LOCAL_LEAK_HOOK.call_once(|| unsafe {
            atexit(alm_trace_leak_check);
        });
    }
    hdr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_tbl_reserve(t: *mut AlmTable, n: i64) {
    if t.is_null() {
        unsafe { alm_trace_set(TRACE_RT_RESERVE_NULL) };
        return;
    }
    let raw = unsafe { t.cast::<RawAlmTable>().read() };
    if !header_is_sane(raw.esize, raw.len, raw.mode) {
        alm_fatal!(
            "alm fatal: alm_tbl_reserve called on freed or corrupted AlmTable {:p} (len={}, esize=0x{:x}, mode={})",
            t,
            raw.len,
            raw.esize,
            raw.mode
        );
    }

    let tbl = unsafe { &mut *t };
    if tbl.mode != TableMode::Dense {
        return;
    }

    if n > tbl.border {
        tbl.border = n;
    }
    unsafe { span_grow(tbl, n) };
}

/// The widened pre-header reserve: `bound + K_eff` as a checked sum.
/// The fast stores downstream trust the reserve blindly (raw GEP, no
/// bounds check), so an overflowing widening must abort loud — a
/// silent skip would leave the span short and the stores writing
/// past it (segfaulted by cases/fill_reserve_offset_wrap_panic.lua
/// before the check existed). Underflow can only reach here on a
/// zero-trip loop (the nonneg gate pins entry >= 0), accepted as a
/// pathological false alarm alongside abort_alloc.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_tbl_reserve_widened(t: *mut AlmTable, n: i64, k: i64) {
    let Some(widened) = n.checked_add(k) else {
        alm_fatal!("alm fatal: reserve overflow (bound={n}, offset={k})");
    };
    unsafe { alm_tbl_reserve(t, widened) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_tbl_len(t: *const AlmTable) -> i64 {
    if t.is_null() {
        return 0;
    }
    unsafe { (*t).border }
}

pub const SPARSE_THRESHOLD: i64 = 100_000;

#[inline(never)]
unsafe fn overflow_map(tbl: &mut AlmTable) -> &mut HashMap<i64, u128> {
    if tbl.sparse_map.is_null() {
        tbl.sparse_map = Box::into_raw(Box::new(HashMap::with_capacity(1)));
    }
    unsafe { &mut *tbl.sparse_map }
}

mod cell_sealed {
    pub trait Sealed {}
}

pub trait CellRepr: Copy + cell_sealed::Sealed {
    const ESIZE: usize;

    const ALIGN: usize;

    unsafe fn load_slot(slot: *const u8) -> Self;

    fn to_lane(self) -> u128;

    fn from_lane(lane: u128) -> Self;

    unsafe fn store_span(slot: *mut u8, cell: Self);

    unsafe fn load_span(slot: *const u8) -> Self;

    unsafe fn store_dst(cell: Self, dst: *mut u8);
}

impl cell_sealed::Sealed for u8 {}
impl cell_sealed::Sealed for u64 {}
impl cell_sealed::Sealed for u128 {}

impl CellRepr for u8 {
    const ESIZE: usize = 1;
    const ALIGN: usize = 8;
    unsafe fn load_slot(slot: *const u8) -> Self {
        unsafe { *slot }
    }
    fn to_lane(self) -> u128 {
        u128::from(self)
    }
    fn from_lane(lane: u128) -> Self {
        (lane & 0xff) as u8
    }
    unsafe fn store_span(slot: *mut u8, cell: Self) {
        unsafe { slot.write(cell) };
    }
    unsafe fn load_span(slot: *const u8) -> Self {
        unsafe { slot.read() }
    }
    unsafe fn store_dst(cell: Self, dst: *mut u8) {
        unsafe { dst.write(cell) };
    }
}

impl CellRepr for u64 {
    const ESIZE: usize = 8;
    const ALIGN: usize = 8;
    unsafe fn load_slot(slot: *const u8) -> Self {
        debug_assert!(slot.addr() & 7 == 0, "u64 slot cell not 8-aligned");
        unsafe { (slot as *const u64).read() }
    }
    fn to_lane(self) -> u128 {
        u128::from(self)
    }
    fn from_lane(lane: u128) -> Self {
        lane as u64
    }
    unsafe fn store_span(slot: *mut u8, cell: Self) {
        unsafe { (slot as *mut u64).write(cell) };
    }
    unsafe fn load_span(slot: *const u8) -> Self {
        unsafe { (slot as *const u64).read() }
    }
    unsafe fn store_dst(cell: Self, dst: *mut u8) {
        debug_assert!(dst.addr() & 7 == 0, "u64 dst cell not 8-aligned");
        unsafe { (dst as *mut u64).write(cell) };
    }
}

impl CellRepr for u128 {
    const ESIZE: usize = 16;
    const ALIGN: usize = 16;
    unsafe fn load_slot(slot: *const u8) -> Self {
        debug_assert!(slot.addr() & 15 == 0, "u128 slot cell not 16-aligned");
        unsafe { (slot as *const u128).read() }
    }
    fn to_lane(self) -> u128 {
        self
    }
    fn from_lane(lane: u128) -> Self {
        lane
    }
    unsafe fn store_span(slot: *mut u8, cell: Self) {
        debug_assert!(slot.addr() & 15 == 0, "u128 span cell not 16-aligned");
        unsafe { (slot as *mut u128).write(cell) };
    }
    unsafe fn load_span(slot: *const u8) -> Self {
        debug_assert!(slot.addr() & 15 == 0, "u128 span cell not 16-aligned");
        unsafe { (slot as *const u128).read() }
    }
    unsafe fn store_dst(cell: Self, dst: *mut u8) {
        debug_assert!(dst.addr() & 15 == 0, "u128 dst cell not 16-aligned");
        unsafe { (dst as *mut u128).write(cell) };
    }
}

const _: () = assert!(u8::ESIZE == 1 && u64::ESIZE == 8 && u128::ESIZE == 16);
const _: () = {
    assert!(u8::ALIGN == 8 && u64::ALIGN == 8 && u128::ALIGN == 16);
    assert!(span_align(u8::ESIZE) == u8::ALIGN);
    assert!(span_align(u64::ESIZE) == u64::ALIGN);
    assert!(span_align(u128::ESIZE) == u128::ALIGN);
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_div_zero_guard(rhs: i64) {
    if rhs == 0 {
        alm_fatal!(
            trace: TRACE_FAIL_DIV_ZERO,
            "alm runtime error: attempt to perform 'n/0' (integer division by zero)"
        );
    }
}

/// The store faces' one null-table death (registry slot 71, the
/// FAIL_NULL_ROW_STORE law): row absence at runtime — base proved a
/// table of tables, this cell null — is the one way a store meets a
/// null table, and every store face shares the death while the read
/// side stays total.
macro_rules! refuse_null_table {
    ($t:expr) => {
        if $t.is_null() {
            alm_fatal!(
                trace: TRACE_FAIL_NULL_ROW_STORE,
                "alm runtime error: attempt to index a nil value (null table row)"
            );
        }
    };
}

/// The byte-pointer read faces' one null answer: a null destination
/// is a no-op read, and a null table zero-fills it — reads stay
/// total (a null row reads the zero lane), never the stores' death.
macro_rules! null_table_reads_zero {
    ($t:expr, $dst:expr, $esize:expr) => {
        if $dst.is_null() {
            return;
        }
        if $t.is_null() {
            unsafe { std::ptr::write_bytes($dst, 0, $esize) };
            return;
        }
    };
}

/// The one esize-dispatch law: the three cell reprs (u8, u64, u128
/// lanes at esize 1, 8, 16) are the whole repr universe, and a header
/// reading any other esize is corrupt or foreign — the face dies
/// naming the esize and its own surface ("has no cell repr" here,
/// "has no register cell face" in the register macro below).
macro_rules! esize_dispatch {
    ($t:expr, $lane:ident ($($args:expr),* $(,)?), $surface:literal) => {
        match unsafe { (&*$t).esize } {
            1 => unsafe { $lane::<u8>($($args),*) },
            8 => unsafe { $lane::<u64>($($args),*) },
            16 => unsafe { $lane::<u128>($($args),*) },
            other => alm_fatal!(
                concat!("alm runtime error: AlmTable esize {} ", $surface),
                other
            ),
        }
    };
}

/// The register faces' dispatch: the i128 lane is a u128 cell, esize
/// 16 only — any other esize dies naming the register face.
macro_rules! esize_register_lane {
    ($t:expr, $body:expr) => {
        match unsafe { (&*$t).esize } {
            16 => $body,
            other => alm_fatal!(
                "alm runtime error: AlmTable esize {} has no register cell face",
                other
            ),
        }
    };
}

/// Declarative generator for the entire exported table read/write ABI surface.
macro_rules! define_table_abi {
    (
        ptr_stores: [ $( $(#[$s_meta:meta])* $s_fn:ident => $s_core:ident ($($s_extra:expr),*) ),* $(,)? ],
        ptr_loads:  [ $( $(#[$l_meta:meta])* $l_fn:ident => $l_core:ident ),* $(,)? ],
        reg_stores: [ $( $(#[$rs_meta:meta])* $rs_fn:ident => $rs_core:ident ($($rs_extra:expr),*) ),* $(,)? ],
        reg_loads:  [ $( $(#[$rl_meta:meta])* $rl_fn:ident => $rl_core:ident ),* $(,)? ],
    ) => {
        $(
            $(#[$s_meta])*
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $s_fn(t: *mut AlmTable, key: i64, val: *const u8) {
                refuse_null_table!(t);
                unsafe fn lane<C: CellRepr>(t: *mut AlmTable, key: i64, val: *const u8) {
                    unsafe { $s_core::<C>(t, key, C::load_slot(val) $(, $s_extra)*) }
                }
                esize_dispatch!(t, lane(t, key, val), "has no cell repr");
            }
        )*
        $(
            $(#[$l_meta])*
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $l_fn(t: *mut AlmTable, key: i64, dst: *mut u8, esize: usize) {
                null_table_reads_zero!(t, dst, esize);
                unsafe fn lane<C: CellRepr>(t: *mut AlmTable, key: i64, dst: *mut u8) {
                    unsafe { C::store_dst($l_core::<C>(t, key), dst) }
                }
                esize_dispatch!(t, lane(t, key, dst), "has no cell repr");
            }
        )*
        $(
            $(#[$rs_meta])*
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $rs_fn(t: *mut AlmTable, key: i64, val: i128) {
                refuse_null_table!(t);
                esize_register_lane!(
                    t,
                    unsafe { $rs_core::<u128>(t, key, val as u128 $(, $rs_extra)*) }
                );
            }
        )*
        $(
            $(#[$rl_meta])*
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn $rl_fn(t: *mut AlmTable, key: i64) -> i128 {
                if t.is_null() {
                    return 0;
                }
                esize_register_lane!(t, unsafe { $rl_core::<u128>(t, key) as i128 })
            }
        )*
    };
}

define_table_abi! {
    ptr_stores: [
        /// The pooled-handle face: interned strings and closure creation
        /// envs. The destination never owns the cell, so a refused store
        /// drops it (ownership notes in docs/lane_single_brain.md).
        alm_tbl_set => set_core(false),

        /// The owned-row face: the IR mints this symbol only for `Tbl`
        /// elem stores, and the static pass has already moved the child
        /// into this store's hands. A refused store reclaims the child —
        /// the symbol itself is the proof, so the refuse path never
        /// consults the destination's header bit (env tables set that bit
        /// for borrowed closure faces, so the bit cannot name ownership).
        alm_tbl_set_row => set_core(true),

        alm_tbl_set_skey => set_skey_core(),
    ],
    ptr_loads: [
        alm_tbl_get => get_core,
        alm_tbl_get_skey => get_skey_core,
    ],
    reg_stores: [
        alm_tbl_set_any => set_core(false),
        alm_tbl_set_any_skey => set_skey_core(),
    ],
    reg_loads: [
        alm_tbl_get_any => get_core,
        alm_tbl_get_any_skey => get_skey_core,
    ],
}

unsafe fn set_core<C: CellRepr>(t: *mut AlmTable, index: i64, cell: C, owns_cell: bool) {
    let tbl = unsafe { &mut *t };

    if index < 0 {
        // A refused store of an owned row must reclaim it: the static
        // pass already moved the child into this store's hands, so
        // dropping the write on the floor would leave the child with
        // zero owners. `owns_cell` arrives minted from the IR's
        // Row/Handle split, so the refuse path and the free path agree
        // on what the cell holds by construction.
        if owns_cell && C::ESIZE == 8 {
            // The lane word is an integer the IR and the span traffic
            // as a value; the reconstitution seam recovers the
            // exposed-provenance pointer the mint declared.
            let child = ptr::with_exposed_provenance_mut::<AlmTable>(cell.to_lane() as usize);
            unsafe { free_tbl(child, &[]) };
        }
        return;
    }

    // The two map-backed lanes, one law: a born-sparse table's record
    // map and a far key's overflow park (allocated lazily, the park
    // seam probes/p22_overflow_park_dense_grow_leak.lua) both stretch
    // the border to the index and ride the word in the map, an owned
    // displaced word reclaiming before the insert wins. The near-key
    // dense envelope falls through to the span below.
    if tbl.mode == TableMode::Sparse || index > tbl.len.saturating_add(SPARSE_THRESHOLD) {
        tbl.border = tbl.border.max(index.saturating_add(1));
        let map = if tbl.mode == TableMode::Sparse {
            unsafe { &mut *tbl.sparse_map }
        } else {
            unsafe { overflow_map(tbl) }
        };
        if let Some(old) = map.insert(index, cell.to_lane())
            && owns_cell
        {
            unsafe { reclaim_displaced_child::<C>(tbl, C::from_lane(old), cell) };
        }
        return;
    }

    tbl.border = tbl.border.max(index.saturating_add(1));
    unsafe { span_grow(tbl, index.wrapping_add(1)) };
    let ptr = unsafe { tbl.data.add(index as usize * C::ESIZE) };
    if owns_cell {
        let old = unsafe { C::load_span(ptr) };
        unsafe { reclaim_displaced_child::<C>(tbl, old, cell) };
    }
    unsafe { C::store_span(ptr, cell) };
    if !tbl.sparse_map.is_null() {
        // The overflow-park seam (probes/p22_overflow_park_dense_grow_leak.lua):
        // a prior write to this index may have parked in the overflow map
        // before tbl.len grew past `index - SPARSE_THRESHOLD` and flipped the
        // write into Branch 3's dense envelope. The parked lane is the slot's
        // first occupant — an owned row the map still owns the last pointer
        // to. Taking `remove`'s result through the same reclaim the span word
        // obeys frees the (idempotent) victim: a parked index's span word is
        // guaranteed 0 and vice versa, so one of the two lanes is always 0,
        // and reclaim_displaced_child's validity gate passes over only the
        // live one.
        let parked = unsafe { (&mut *tbl.sparse_map).remove(&index) };
        if owns_cell && let Some(parked) = parked {
            unsafe { reclaim_displaced_child::<C>(tbl, C::from_lane(parked), cell) };
        }
    }
}

/// One lane walk for free_tbl's twin child walks (the dense span and
/// the sparse/overflow map) and the displaced-child reclaim's tail.
/// The lane words are integer lanes the span and the map traffic as
/// values (store_span writes the lane, never the pointer), so each
/// reconstitutes through the exposed-provenance seam — a
/// pointer-typed read of an integer-written slot carries no
/// provenance (Miri: dangling [noalloc]) — and a lane reading as a
/// live table address frees under the keep veto; zero words and
/// kernel islands never pass valid_slot_addr.
unsafe fn reclaim_lane(lane: u128, keeps: &[*mut AlmTable]) {
    let child = ptr::with_exposed_provenance_mut::<AlmTable>(lane as usize);
    if valid_slot_addr(child as usize) {
        unsafe { alm_trace_set(TRACE_RT_FREE_CHILD) };
        unsafe { free_tbl(child, keeps) };
    }
}

/// The displaced-child reclaim. An owned-row replace is the slot's
/// second word of a table lane: zero words and non-pointer words skip
/// (`valid_slot_addr`, the same consult free_tbl's child walk makes),
/// and a self-rewrite holds the old lane, so the free never fires for
/// it — resetting a slot to its own row is not a replace.
unsafe fn reclaim_displaced_child<C: CellRepr>(tbl: &AlmTable, old_slot: C, new_cell: C) {
    if C::ESIZE != 8 || tbl.contains_tables == 0 {
        return;
    }
    let old_lane = old_slot.to_lane();
    if old_lane == new_cell.to_lane() {
        return;
    }
    unsafe { reclaim_lane(old_lane, &[]) };
}

/// The stored-slot predicate: a lane reads as a live table pointer when
/// it sits in user space and is 8-byte aligned; zero words (unwritten
/// slots) and kernel islands fail it. free_tbl's child walk and the
/// replace reclaim consult the one predicate.
///
/// The 1 MiB floor is an allocator-layout assumption (system heap bases
/// sit far above it; zero words and small-integer lanes sit below).
/// Miri's heap base sits BELOW the floor (~1 MB in the runs), so under
/// the interpreter the floor yields and the zero-lane + kernel-island
/// laws carry the rejection alone.
fn valid_slot_addr(addr: usize) -> bool {
    #[cfg(target_pointer_width = "64")]
    let is_user_space = addr < 0x0000_7FFF_FFFF_FFFF;
    #[cfg(not(target_pointer_width = "64"))]
    let is_user_space = true;

    #[cfg(miri)]
    {
        addr >= 0x1000 && is_user_space && (addr & 7) == 0
    }
    #[cfg(not(miri))]
    {
        addr > 0x0100_0000 && is_user_space && (addr & 7) == 0
    }
}

unsafe fn set_skey_core<C: CellRepr>(t: *mut AlmTable, key: i64, cell: C) {
    debug_assert!(is_str_key(key), "skey store outside the reserved range");
    let tbl = unsafe { &mut *t };
    unsafe { overflow_map(tbl).insert(key, cell.to_lane()) };
}

unsafe fn get_core<C: CellRepr>(t: *mut AlmTable, index: i64) -> C {
    let tbl = unsafe { &mut *t };

    if index < 0 {
        return C::from_lane(0);
    }

    if tbl.mode == TableMode::Sparse {
        let map = unsafe { &mut *tbl.sparse_map };
        return match map.get(&index) {
            Some(&lane) => C::from_lane(lane),
            None => C::from_lane(0),
        };
    }

    if !tbl.sparse_map.is_null()
        && let Some(&lane) = unsafe { (&*tbl.sparse_map).get(&index) }
    {
        return C::from_lane(lane);
    }
    if index >= tbl.len {
        return C::from_lane(0);
    }
    let ptr = unsafe { tbl.data.add(index as usize * C::ESIZE) };
    unsafe { C::load_span(ptr) }
}

unsafe fn get_skey_core<C: CellRepr>(t: *mut AlmTable, key: i64) -> C {
    debug_assert!(is_str_key(key), "skey read outside the reserved range");
    let tbl = unsafe { &mut *t };
    if !tbl.sparse_map.is_null()
        && let Some(&lane) = unsafe { (&*tbl.sparse_map).get(&key) }
    {
        return C::from_lane(lane);
    }
    C::from_lane(0)
}

unsafe fn span_grow(tbl: &mut AlmTable, want: i64) {
    if want < 0 || want <= tbl.len {
        return;
    }
    let new_len = want.max(tbl.len.wrapping_mul(2)).max(8);
    let new_bytes = bytes_of(new_len, tbl.esize).unwrap_or_else(|| abort_alloc(usize::MAX));
    if tbl.reserve != 0 {
        unsafe { grow_vm(tbl, new_bytes) };
        tbl.len = new_len;
        return;
    }
    if new_bytes >= GRADUATE_AT && vm_enabled() {
        unsafe { graduate(tbl, new_bytes) };
        tbl.len = new_len;
        return;
    }
    let new = elem_layout(new_len, tbl.esize);
    let buf = if tbl.data.is_null() {
        unsafe { alloc_zeroed(new) }
    } else {
        let old = elem_layout(tbl.len, tbl.esize);
        let grown = unsafe { realloc(tbl.data.cast(), old, new.size()) };

        if grown.is_null() {
            abort_alloc(new.size());
        }

        unsafe {
            grown
                .byte_offset((tbl.len * tbl.esize as i64) as isize)
                .write_bytes(0, (new_len - tbl.len) as usize * tbl.esize);
        }
        grown
    };
    tbl.data = buf;
    tbl.len = new_len;
}

const GRADUATE_AT: usize = 1 << 20;
const RESERVE_FLOOR: usize = 1 << 26;
const PAGE: usize = 4096;

const PROT_NONE: i32 = 0x0;
const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_NORESERVE: i32 = 0x4000;
const MREMAP_MAYMOVE: i32 = 1;

unsafe extern "C" {
    fn mmap(addr: *mut c_void, len: usize, prot: i32, flags: i32, fd: i32, off: i64)
    -> *mut c_void;
    fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
    fn mremap(addr: *mut c_void, old_len: usize, new_len: usize, flags: i32, ...) -> *mut c_void;
    fn atexit(cb: unsafe extern "C" fn()) -> i32;
}

fn reserve_for(bytes: usize) -> usize {
    bytes
        .max(RESERVE_FLOOR)
        .checked_next_power_of_two()
        .unwrap_or_else(|| abort_alloc(usize::MAX))
}

fn is_map_failed(p: *mut c_void) -> bool {
    p.addr() == usize::MAX
}

fn round_up_page(n: usize) -> usize {
    (n + PAGE - 1) & !(PAGE - 1)
}

fn vm_enabled() -> bool {
    static STATE: OnceLock<bool> = OnceLock::new();
    *STATE.get_or_init(|| !std::env::var_os("ALM_VM").is_some_and(|v| v == "off" || v == "0"))
}

unsafe fn graduate(tbl: &mut AlmTable, new_bytes: usize) {
    let reserve = reserve_for(new_bytes);
    let vm = unsafe {
        mmap(
            ptr::null_mut(),
            reserve,
            PROT_NONE,
            MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE,
            -1,
            0,
        )
    };
    if is_map_failed(vm) {
        abort_alloc(reserve);
    }
    let commit = round_up_page(new_bytes);
    if unsafe { mprotect(vm, commit, PROT_READ | PROT_WRITE) } != 0 {
        unsafe { munmap(vm, reserve) };
        abort_alloc(commit);
    }
    if !tbl.data.is_null() {
        unsafe {
            ptr::copy_nonoverlapping(
                tbl.data,
                vm.cast::<u8>(),
                elem_layout(tbl.len, tbl.esize).size(),
            );
            dealloc(tbl.data, elem_layout(tbl.len, tbl.esize));
        }
    }
    tbl.data = vm.cast::<u8>();
    tbl.reserve = reserve;
}

unsafe fn grow_vm(tbl: &mut AlmTable, new_bytes: usize) {
    if new_bytes > tbl.reserve {
        let new_reserve = reserve_for(new_bytes);
        let moved = unsafe { mremap(tbl.data.cast(), tbl.reserve, new_reserve, MREMAP_MAYMOVE) };
        if is_map_failed(moved) {
            abort_alloc(new_reserve);
        }
        tbl.data = moved.cast::<u8>();
        tbl.reserve = new_reserve;
    }
    let commit = round_up_page(new_bytes);
    if unsafe { mprotect(tbl.data.cast(), commit, PROT_READ | PROT_WRITE) } != 0 {
        abort_alloc(commit);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_tbl_free(t: *mut AlmTable) {
    unsafe { free_tbl(t, &[]) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_tbl_free_except(t: *mut AlmTable, keep: *mut AlmTable) {
    unsafe { free_tbl(t, &[keep]) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_tbl_free_except_n(
    t: *mut AlmTable,
    keeps: *const *mut AlmTable,
    n: usize,
) {
    if keeps.is_null() {
        unsafe { free_tbl(t, &[]) };
        return;
    }
    let slice = unsafe { std::slice::from_raw_parts(keeps, n) };
    unsafe { free_tbl(t, slice) };
}

const FREE_POISON_ESIZE: usize = 0xDEAD_F8EE_DEAD_F8EE;

/// The one header-corruption law, consulted by both freed-header
/// sniffs: a live header reads esize as one of the three cell reprs
/// (which already excludes the free poison — it is none of 1, 8, 16),
/// mode as one of the two TableModes, and a non-negative len.
/// alm_tbl_reserve refuses anything not sane through it;
/// validate_header_before_free folds it into the fuller pre-free
/// audit (elem_kind, the bytes_of product, the span shape) after its
/// own DOUBLE FREE diagnosis splits the poison out first.
fn header_is_sane(esize: usize, len: i64, mode: u8) -> bool {
    esize != FREE_POISON_ESIZE && matches!(esize, 1 | 8 | 16) && mode <= 1 && len >= 0
}

unsafe fn validate_header_before_free(t: *mut AlmTable, prev_allocs: usize) {
    let raw = unsafe { t.cast::<RawAlmTable>().read() };

    if raw.esize == FREE_POISON_ESIZE {
        alm_fatal!(
            "alm fatal: DOUBLE FREE of AlmTable header {:p} \
             (tcache next/data={:p}, tcache_key/len=0x{:016x} [{}], prev_alloc_count={})",
            t,
            raw.data,
            raw.len as u64,
            raw.len,
            prev_allocs
        );
    }

    let valid_elem_kind = raw.elem_kind <= ALM_ELEM_UNDECIDED;
    let valid_len = bytes_of(raw.len, raw.esize).is_some();
    let valid_span = (raw.len == 0 && raw.data.is_null())
        || (raw.len > 0 && !raw.data.is_null())
        || raw.mode == 1;

    if prev_allocs == 0
        || !header_is_sane(raw.esize, raw.len, raw.mode)
        || !valid_elem_kind
        || !valid_len
        || !valid_span
    {
        alm_fatal!(
            "alm fatal: corrupted or already-freed AlmTable header at {:p} during free_tbl:\n  \
             prev_alloc_count = {}\n  \
             @0  data            = {:p}\n  \
             @8  len             = {} (0x{:016x})\n  \
             @16 reserve         = {} (0x{:x})\n  \
             @24 esize           = {} (0x{:x})\n  \
             @32 mode            = {}\n  \
             @33 contains_tables = {}\n  \
             @34 elem_kind       = {}\n  \
             @40 sparse_map      = {:p}\n  \
             @48 border          = {}",
            t,
            prev_allocs,
            raw.data,
            raw.len,
            raw.len as u64,
            raw.reserve,
            raw.reserve,
            raw.esize,
            raw.esize,
            raw.mode,
            raw.contains_tables,
            raw.elem_kind,
            raw.sparse_map,
            raw.border,
        );
    }
}

unsafe fn free_tbl(t: *mut AlmTable, keeps: &[*mut AlmTable]) {
    if t.is_null() || keeps.contains(&t) {
        return;
    }
    unsafe { alm_trace_set(TRACE_RT_FREE) };
    let prev_allocs = ALLOC_COUNT.fetch_sub(1, Ordering::Relaxed);
    unsafe { mirror_census(current_exec_id()) };
    unsafe { validate_header_before_free(t, prev_allocs) };
    let mut tbl = unsafe { Box::from_raw(t) };

    let esize = tbl.esize;
    unsafe { ptr::write_volatile(&mut tbl.esize, FREE_POISON_ESIZE) };

    if tbl.contains_tables != 0 {
        if tbl.mode == TableMode::Dense && !tbl.data.is_null() {
            let lanes =
                unsafe { std::slice::from_raw_parts(tbl.data.cast::<u64>(), tbl.len as usize) };
            for &lane in lanes {
                unsafe { reclaim_lane(u128::from(lane), keeps) };
            }
        }
        if !tbl.sparse_map.is_null() {
            for &lane in unsafe { (&*tbl.sparse_map).values() } {
                unsafe { reclaim_lane(lane, keeps) };
            }
        }
    }

    if !tbl.sparse_map.is_null() {
        let _ = unsafe {
            Box::from_raw(
                tbl.sparse_map
                    .cast::<std::collections::HashMap<i64, u128>>(),
            )
        };
    }
    if tbl.mode != TableMode::Sparse && !tbl.data.is_null() {
        if tbl.reserve != 0 {
            unsafe { munmap(tbl.data.cast(), tbl.reserve) };
        } else {
            unsafe { dealloc(tbl.data, elem_layout(tbl.len, esize)) };
        }
    }

    unsafe {
        ptr::write_volatile(&mut tbl.data, ptr::null_mut());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_print_int(val: i64) {
    print!("{val}");
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_print_float(val: f64) {
    print!("{val}");
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_print_bool(val: bool) {
    print!("{val}");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_print_string(val: *const u8) {
    if val.is_null() {
        return;
    }
    let bytes = unsafe { cstr_bytes(val) };
    print!("{}", String::from_utf8_lossy(bytes));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_str_len(val: *const u8) -> i64 {
    if val.is_null() {
        return 0;
    }
    unsafe { cstr_bytes(val).len() as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_str_register(ptrs: *const *const u8, n: usize) {
    unsafe { vx_str_register(ptrs as *mut *const u8, n) };
}

/// `..` at runtime: join the two pooled handles and intern the result,
/// so the product is pointer-identical to any literal with the same
/// bytes. Literals carry no NULs (the lexer rejects them) and pooled
/// strings are NUL-terminated, so the C-string read is the whole law.
/// A null handle reads as empty — concat with nil is the checker's
/// rejection to make, not this seam's.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_str_concat(l: *const u8, r: *const u8) -> *const u8 {
    let l_bytes = if l.is_null() {
        &[][..]
    } else {
        unsafe { cstr_bytes(l) }
    };
    let r_bytes = if r.is_null() {
        &[][..]
    } else {
        unsafe { cstr_bytes(r) }
    };
    let joined: Vec<u8> = [l_bytes, r_bytes].concat();
    unsafe { alm_str_intern(joined.as_ptr(), joined.len()) }
}

/// `assert`'s fail path: a false condition raises the interned message
/// (the default when none was given) — one death, both messages ride
/// the pool. The condition rides widened (i1 -> i64) across the C
/// boundary; zero is false, everything else is true.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_assert_guard(cond: i64, msg: *const u8) {
    if cond == 0 {
        let text = if msg.is_null() {
            String::from("assertion failed!")
        } else {
            String::from_utf8_lossy(unsafe { cstr_bytes(msg) }).into_owned()
        };
        alm_fatal!(
            trace: TRACE_FAIL_ASSERT,
            "alm runtime error: assertion failed! {text}"
        );
    }
}

/// `error`'s unconditional raise: the interned message through the
/// same trap as assert's fail path — one machinery, two verbs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_error_raise(msg: *const u8) {
    let text = if msg.is_null() {
        String::from("(no message)")
    } else {
        String::from_utf8_lossy(unsafe { cstr_bytes(msg) }).into_owned()
    };
    alm_fatal!(
        trace: TRACE_FAIL_ERROR_RAISED,
        "alm runtime error: uncaught error: {text}"
    );
}

/// `tonumber`'s runtime half: the digits-only parse. No fallback, no
/// nil answer — non-digit bytes (or an empty handle, or a value that
/// does not fit an i64) are the loud death; a literal non-digit is the
/// checker's named rejection, this is the same law at runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_str_tonumber(s: *const u8) -> i64 {
    let bytes = if s.is_null() {
        &[][..]
    } else {
        unsafe { cstr_bytes(s) }
    };
    let parse_failure = || {
        alm_fatal!(
            trace: TRACE_FAIL_TONUMBER,
            "alm runtime error: tonumber is a digits-only parse — got '{}'",
            String::from_utf8_lossy(bytes)
        );
    };
    if bytes.is_empty() || !bytes.iter().all(|b| b.is_ascii_digit()) {
        parse_failure();
    }
    match std::str::from_utf8(bytes)
        .ok()
        .and_then(|t| t.parse::<i64>().ok())
    {
        Some(n) => n,
        None => parse_failure(),
    }
}

/// The Miri stand-in for the C-side intern pool: the pool is a foreign
/// function the interpreter cannot execute, so the law it must preserve
/// is re-stated here — content dedup, an 8-byte str-key prefix before
/// the returned bytes, a NUL terminator. `alm_str_key`'s unaligned
/// prefix read is the ABI contract this layout answers to. The entries
/// live in the static pool itself (a pushed Vec's buffer never moves),
/// so the process exit drains them — no Box::leak to trip Miri's leak
/// check, no immortality claim to justify.
#[cfg(miri)]
unsafe fn miri_intern(s: *const u8, len: usize) -> *const u8 {
    use std::sync::Mutex;
    static POOL: Mutex<Vec<(usize, Vec<u64>)>> = Mutex::new(Vec::new());
    let bytes = unsafe { std::slice::from_raw_parts(s, len) }.to_vec();
    let mut pool = POOL.lock().unwrap();
    let at = pool.iter().position(|(elen, words)| {
        *elen == len && {
            let raw =
                unsafe { std::slice::from_raw_parts(words.as_ptr().add(1).cast::<u8>(), len) };
            raw == bytes.as_slice()
        }
    });
    let entry = if let Some(at) = at {
        &pool[at].1
    } else {
        let total_bytes = 8 + len + 1;
        let mut words = vec![0u64; total_bytes.div_ceil(8)];
        words[0] = str_key_of_bytes(&bytes) as u64;
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), words.as_mut_ptr().add(1).cast::<u8>(), len);
        }
        pool.push((len, words));
        &pool.last().unwrap().1
    };
    let ptr = unsafe { entry.as_ptr().add(1).cast::<u8>() };
    ptr.expose_provenance();
    ptr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_str_intern(s: *const u8, len: usize) -> *const u8 {
    if len == 0 {
        // The one-empty law's Rust face: the empty string interns as the
        // null handle — the one value the zero lane already reads as ""
        // (len/concat/print) and the key face mints as EMPTY_STR_KEY.
        // No pooled "" ever exists, so register identity (`==`) and
        // keying agree on the empty with zero normalization.
        return ptr::null();
    }
    #[cfg(miri)]
    {
        return unsafe { miri_intern(s, len) };
    }
    #[cfg(not(miri))]
    unsafe {
        vx_str_intern(s, len)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_str_key(s: *const u8) -> i64 {
    if s.is_null() {
        return EMPTY_STR_KEY;
    }
    let key_ptr = unsafe { (s as *const i64).sub(1) };
    debug_assert!(key_ptr.addr() & 7 == 0, "alm_str_key prefix not 8-aligned");
    unsafe { key_ptr.read() }
}

pub const STR_KEY_TAG: i64 = i64::MIN;

/// The null handle's key: a NULL string reads as empty everywhere else
/// in the runtime (len, concat, print), so it keys as the empty string
/// too — a missing record cell and `t[""]` are one slot, and the skey
/// lanes never see an untagged value. Before this law the null handle
/// answered -1, an occupant outside the minted range that turned every
/// missing key into one hidden shared slot.
pub const EMPTY_STR_KEY: i64 = str_key_of_bytes(b"");

#[inline]
pub fn is_str_key(index: i64) -> bool {
    (index & (3i64 << 62)) == STR_KEY_TAG
}

pub const fn str_key_of_bytes(bytes: &[u8]) -> i64 {
    let mut h: u64 = 14695981039346656037;
    let mut i = 0;
    while i < bytes.len() {
        h ^= bytes[i] as u64;
        h = h.wrapping_mul(1099511628211);
        i += 1;
    }
    (STR_KEY_TAG as u64 | (h & 0x3FFF_FFFF_FFFF_FFFF)) as i64
}

#[inline]
fn any_pack(tag: i32, payload: u64) -> i128 {
    ((tag as i64 as i128) << 64) | (payload as i128)
}

#[inline]
fn any_tag(v: i128) -> i32 {
    (v >> 64) as i32
}

#[inline]
fn any_payload(v: i128) -> u64 {
    v as u64
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_any_print(v: i128) {
    match any_tag(v) {
        ALM_ARG_INT => print!("{}", any_payload(v) as i64),
        ALM_ARG_FLOAT => print!("{}", f64::from_bits(any_payload(v))),
        ALM_ARG_BOOL => print!("{}", any_payload(v) != 0),
        ALM_ARG_STRING => {
            let p = ptr::with_exposed_provenance::<u8>(any_payload(v) as usize);
            if !p.is_null() {
                let bytes = unsafe { cstr_bytes(p) };
                print!("{}", String::from_utf8_lossy(bytes));
            }
        }
        _ => print!("{}", any_payload(v) as i64),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_any_eq(l: i128, r: i128) -> i32 {
    i32::from(l == r)
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_any_len(v: i128) -> i64 {
    match any_tag(v) {
        ALM_ARG_STRING => {
            let p = ptr::with_exposed_provenance::<u8>(any_payload(v) as usize);
            if p.is_null() {
                return 0;
            }
            unsafe { alm_str_len(p) }
        }
        k => alm_fatal!(
            trace: TRACE_FAIL_ANY_LEN,
            "alm runtime error: alm_any_len: '#' demands a string cell, got kind {} ({})",
            k,
            arg_kind_short(k)
        ),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_any_from_int(v: i64) -> i128 {
    any_pack(ALM_ARG_INT, v as u64)
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_any_from_float(v: f64) -> i128 {
    any_pack(ALM_ARG_FLOAT, v.to_bits())
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_any_from_bool(v: bool) -> i128 {
    any_pack(ALM_ARG_BOOL, u64::from(v))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_any_from_str(p: *const u8) -> i128 {
    any_pack(ALM_ARG_STRING, p as u64)
}

pub enum AnyWord {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str,
}

pub fn classify_word(w: &[u8]) -> AnyWord {
    if let Ok(s) = std::str::from_utf8(w) {
        if let Ok(v) = s.parse::<i64>() {
            return AnyWord::Int(v);
        }
        if let Ok(v) = s.parse::<f64>() {
            return AnyWord::Float(v);
        }
        match s {
            "true" => return AnyWord::Bool(true),
            "false" => return AnyWord::Bool(false),
            _ => {}
        }
    }
    AnyWord::Str
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_any_pack(tag: i32, payload: u64) -> i128 {
    any_pack(tag, payload)
}

unsafe fn any_of_word(w: &[u8]) -> i128 {
    match classify_word(w) {
        AnyWord::Int(v) => any_pack(ALM_ARG_INT, v as u64),
        AnyWord::Float(v) => any_pack(ALM_ARG_FLOAT, v.to_bits()),
        AnyWord::Bool(v) => any_pack(ALM_ARG_BOOL, u64::from(v)),
        AnyWord::Str => {
            let p = unsafe { alm_str_intern(w.as_ptr(), w.len()) };
            any_pack(ALM_ARG_STRING, p as u64)
        }
    }
}

#[inline]
unsafe fn cstr_bytes(p: *const u8) -> &'static [u8] {
    unsafe { CStr::from_ptr(p.cast()).to_bytes() }
}

pub const ALM_ARG_NONE: i32 = -1;
pub const ALM_ELEM_TABLE: u8 = 5;
pub const ALM_ELEM_UNDECIDED: u8 = 6;

/// Single source of truth for ALM argument kinds, element mappings,
/// byte widths, and diagnostic labels.
macro_rules! define_alm_kinds {
    ($(
        $const_name:ident = $id:literal => {
            elem: $elem:ident,
            esize: $esize:literal,
            short: $short:literal,
            words: $words:literal,
            table: $table:literal $(,)?
        }
    ),* $(,)?) => {
        $( pub const $const_name: i32 = $id; )*

        pub fn arg_kind_of(elem: &crate::AlmElem) -> i32 {
            match elem {
                $( crate::AlmElem::$elem => $const_name, )*
            }
        }

        pub fn elem_of_kind(kind: i32) -> Option<crate::AlmElem> {
            match kind {
                $( $const_name => Some(crate::AlmElem::$elem), )*
                _ => None,
            }
        }

        const fn arg_kind_esize(kind: i32) -> usize {
            match kind {
                $( $const_name => $esize, )*
                _ => 8,
            }
        }

        const fn arg_kind_short(kind: i32) -> &'static str {
            match kind {
                $( $const_name => $short, )*
                _ => "unknown",
            }
        }

        const fn arg_kind_words(kind: i32) -> &'static str {
            match kind {
                $( $const_name => $words, )*
                _ => "integers (64-bit)",
            }
        }

        const fn arg_kind_table(kind: i32) -> &'static str {
            match kind {
                $( $const_name => $table, )*
                _ => "Table<Integer>",
            }
        }
    };
}

define_alm_kinds! {
    ALM_ARG_INT = 0 => {
        elem: Integer,
        esize: 8,
        short: "int",
        words: "integers (64-bit)",
        table: "Table<Integer>",
    },
    ALM_ARG_FLOAT = 1 => {
        elem: Float,
        esize: 8,
        short: "float",
        words: "numbers (64-bit floats)",
        table: "Table<Float>",
    },
    ALM_ARG_BOOL = 2 => {
        elem: Boolean,
        esize: 1,
        short: "bool",
        words: "booleans ('true'/'false')",
        table: "Table<Boolean>",
    },
    ALM_ARG_STRING = 3 => {
        elem: String,
        esize: 8,
        short: "string",
        words: "strings (any word)",
        table: "Table<String>",
    },
    ALM_ARG_ANY = 4 => {
        elem: Any,
        esize: 16,
        short: "any",
        words: "int, float, bool, or string words",
        table: "Table<Any>",
    },
}

unsafe fn set_word(t: *mut AlmTable, i: i64, w: &[u8], kind: i32) -> Result<(), ()> {
    match kind {
        ALM_ARG_INT => {
            let v: i64 = String::from_utf8_lossy(w).parse().map_err(|_| ())?;
            unsafe { alm_tbl_set(t, i, (&v as *const i64).cast()) };
        }
        ALM_ARG_FLOAT => {
            let v: f64 = String::from_utf8_lossy(w).parse().map_err(|_| ())?;
            unsafe { alm_tbl_set(t, i, (&v as *const f64).cast()) };
        }
        ALM_ARG_BOOL => {
            let v = match w {
                b"true" => 1u8,
                b"false" => 0u8,
                _ => return Err(()),
            };
            unsafe { alm_tbl_set(t, i, (&v as *const u8).cast()) };
        }
        ALM_ARG_STRING => {
            let p = unsafe { alm_str_intern(w.as_ptr(), w.len()) };
            unsafe { alm_tbl_set(t, i, (&p as *const *const u8).cast()) };
        }
        ALM_ARG_ANY => {
            let v = unsafe { any_of_word(w) };
            unsafe { alm_tbl_set_any(t, i, v) };
        }
        _ => return Err(()),
    }
    Ok(())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_exec_main(
    argc: i32,
    argv: *const *const u8,
    kind: i32,
    exec: unsafe extern "C" fn(*mut AlmTable) -> *mut AlmTable,
) -> i32 {
    IS_EXE.store(true, Ordering::Relaxed);
    let id = exec_id();
    let rc = unsafe { exec_main_inner(argc, argv, kind, exec) };
    if id >= 0 {
        unsafe { vx_exec_state_set(id, 3) };
    }
    rc
}

unsafe fn exec_main_inner(
    argc: i32,
    argv: *const *const u8,
    kind: i32,
    exec: unsafe extern "C" fn(*mut AlmTable) -> *mut AlmTable,
) -> i32 {
    if kind == ALM_ARG_NONE {
        let result = unsafe { exec(std::ptr::null_mut()) };
        unsafe { alm_tbl_free(result) };
        return 0;
    }
    let mut words: Vec<&[u8]> = Vec::new();
    if !argv.is_null() {
        for i in 1..(argc.max(0) as usize) {
            let p = unsafe { *argv.add(i) };
            if !p.is_null() {
                words.push(unsafe { cstr_bytes(p) });
            }
        }
    }
    let esize = arg_kind_esize(kind);
    let args = unsafe { alm_tbl_new(esize, ((kind as u8) & 0x07) << 1) };
    for (i, w) in words.iter().enumerate() {
        if unsafe { set_word(args, i as i64, w, kind) }.is_err() {
            eprintln!(
                "alm error: boundary args must be {}, got '{}' — the arg table is {}",
                arg_kind_words(kind),
                String::from_utf8_lossy(w),
                arg_kind_table(kind),
            );
            unsafe { alm_tbl_free(args) };
            return 1;
        }
    }
    let result = unsafe { exec(args) };
    unsafe {
        if result != args {
            alm_tbl_free(args);
        }
        alm_tbl_free(result);
    }
    0
}

const RTLD_NOW: i32 = 0x2;
const RTLD_LOCAL: i32 = 0x1;

unsafe extern "C" {
    fn dlopen(filename: *const i8, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const i8) -> *mut c_void;
    fn dlerror() -> *const i8;
}

unsafe fn ffi_open_path(path: *const u8) -> Vec<u8> {
    let bytes = unsafe { cstr_bytes(path) };
    if bytes.contains(&b'/') {
        return Vec::new();
    }
    let mut prefixed = Vec::with_capacity(bytes.len() + 3);
    prefixed.push(b'.');
    prefixed.push(b'/');
    prefixed.extend_from_slice(bytes);
    prefixed.push(0);
    prefixed
}

unsafe fn ffi_dlsym_or_die(lib: *mut c_void, name: *const u8) -> *mut c_void {
    let sym = unsafe { dlsym(lib, name.cast()) };
    if sym.is_null() {
        let shown = if name.is_null() {
            "(null)".to_string()
        } else {
            String::from_utf8_lossy(unsafe { cstr_bytes(name) }).into_owned()
        };
        alm_fatal!(
            trace: TRACE_FAIL_FFI_SYM,
            "alm fatal: ffi could not find symbol '{shown}' in the loaded module"
        );
    }
    sym
}

/// Typed `dlsym` resolver with null-library guard.
unsafe fn ffi_fn<F: Copy>(lib: *mut c_void, sym: *const u8, null_msg: &str) -> F {
    if lib.is_null() {
        alm_fatal!("{null_msg}");
    }
    let raw = unsafe { ffi_dlsym_or_die(lib, sym) };
    unsafe { std::mem::transmute_copy::<*mut c_void, F>(&raw) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_ffi_load(path: *const u8) -> *mut c_void {
    if path.is_null() {
        alm_fatal!("alm fatal: ffi.load called with a null path");
    }
    let prefixed = unsafe { ffi_open_path(path) };
    let direct = prefixed.is_empty();
    let handle = unsafe {
        dlopen(
            if direct {
                path.cast()
            } else {
                prefixed.as_ptr().cast()
            },
            RTLD_NOW | RTLD_LOCAL,
        )
    };
    if handle.is_null() {
        let err = unsafe {
            let e = dlerror();
            if e.is_null() {
                String::new()
            } else {
                String::from_utf8_lossy(cstr_bytes(e.cast())).into_owned()
            }
        };
        alm_fatal!(
            trace: TRACE_FAIL_FFI_LOAD,
            "alm fatal: ffi.load could not open '{}' ({err})",
            String::from_utf8_lossy(unsafe { cstr_bytes(path) }),
        );
    }
    unsafe { alm_trace_set(TRACE_RT_FFI_LOAD) };
    handle
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_ffi_arg_kind(lib: *mut c_void) -> i64 {
    let f = unsafe {
        ffi_fn::<unsafe extern "C" fn() -> i32>(
            lib,
            c"alm_arg_kind".as_ptr().cast(),
            "alm fatal: ffi use of a null library handle",
        )
    };
    unsafe { f() as i64 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_ffi_call(lib: *mut c_void, sym: *const u8, arg: *mut AlmTable) {
    let exec = unsafe {
        ffi_fn::<unsafe extern "C" fn(*mut AlmTable) -> *mut AlmTable>(
            lib,
            sym,
            "alm fatal: ffi call on a null library handle",
        )
    };
    let result = unsafe { exec(arg) };
    if result.is_null() || result == arg {
        return;
    }
    let free_fn = unsafe {
        ffi_fn::<unsafe extern "C" fn(*mut AlmTable)>(
            lib,
            c"alm_tbl_free".as_ptr().cast(),
            "alm fatal: ffi call on a null library handle",
        )
    };
    unsafe { free_fn(result) };
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_print_sep() {
    print!("\t");
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_print_nl() {
    println!();
}

#[unsafe(no_mangle)]
pub extern "C" fn sys_alloc_count() -> i64 {
    ALLOC_COUNT.load(Ordering::Relaxed) as i64
}

#[unsafe(no_mangle)]
pub extern "C" fn sys_exec_count() -> i64 {
    unsafe { vx_exec_count() as i64 }
}

#[unsafe(no_mangle)]
pub extern "C" fn alm_rt_exec_id() -> i32 {
    current_exec_id()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_coro_create(lib: *mut c_void) -> i64 {
    let id = unsafe { vx_coro_create(lib) };
    if id < 0 {
        unsafe { alm_trace_set(TRACE_FAIL_CORO_CREATE) };
    }
    id as i64
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_coro_resume(co: i64, t: *mut AlmTable) -> i64 {
    let id = co as i32;
    if id < 0 {
        alm_fatal!(
            trace: TRACE_FAIL_CORO_CREATE,
            "alm fatal: async.resume on handle {co} — the async registry \
             was exhausted at create time (all lanes live); test \
             async.create's result before resuming"
        );
    }
    if unsafe { vx_coro_status(id) } == 1 && !t.is_null() {
        let mut lanes = [0u64; 3];
        for (i, lane) in lanes.iter_mut().enumerate() {
            let mut buf: u128 = 0;
            unsafe { alm_tbl_get(t, i as i64, ptr::addr_of_mut!(buf).cast(), 16) };
            *lane = buf as u64;
        }
        unsafe { vx_coro_send(id, lanes[0], lanes[1], lanes[2]) };
    }
    let mut done: i32 = 0;
    let value = unsafe {
        vx_coro_start(id);
        alm_trace_set(TRACE_CORO_SPAWN);
        vx_coro_pull(id, &mut done)
    };
    unsafe {
        alm_trace_set(if done != 0 {
            TRACE_CORO_DEAD
        } else {
            TRACE_CORO_RESUME
        })
    };
    if done != 0 { i64::MIN } else { value }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_coro_yield(value: i64) -> i64 {
    let r = unsafe { vx_coro_yield(value as u64) };
    if r == -2 {
        alm_fatal!(
            trace: TRACE_FAIL_CORO_YIELD_RESERVED,
            "alm fatal: async.yield of {value} — the drain word (i64::MIN) is \
             reserved: a drained lane answers it, and only a drained lane, at \
             async.resume; stream any other i64"
        );
    }
    unsafe {
        alm_trace_set(if r < 0 {
            TRACE_CORO_YIELD_DROPPED
        } else {
            TRACE_CORO_YIELD
        })
    };
    r
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_coro_status(co: i64) -> *const u8 {
    let word: &[u8] = match unsafe { vx_coro_status(co as i32) } {
        1 => b"suspended",
        2 => b"running",
        _ => b"dead",
    };
    unsafe { alm_str_intern(word.as_ptr(), word.len()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn alm_coro_close(co: i64) -> i64 {
    unsafe { alm_trace_set(TRACE_CORO_CLOSE) };
    unsafe { vx_coro_join(co as i32) };
    0
}

#[cfg(test)]
mod strkey_tests {
    use super::*;

    #[test]
    fn refused_row_store_reclaims_the_child() {
        // The ownership contract: a store the destination refuses (a
        // negative physical key no-ops) must free the incoming row --
        // static ownership already moved it into the store's hands, so
        // nobody else holds it. The row symbol itself is the proof;
        // the free poisons the child's header, which is race-free
        // proof against parallel tests (ALLOC_COUNT itself is
        // process-wide).
        let parent = unsafe { alm_tbl_new(8, 0x80) };
        let child = unsafe { alm_tbl_new(8, 0) };

        let slot: *const u8 = std::ptr::addr_of!(child).cast();
        unsafe { alm_tbl_set_row(parent, -1, slot) };

        // The poison read is a real-allocator law: the header's memory
        // is freed, and the detector reads it only because the system
        // allocator recycles lazily. Under Miri a freed read is UB by
        // definition, so the interpreter proves the reclaim through the
        // laws around it (the refuse no-growth, the parent's exit free
        // without a second free of the child).
        #[cfg(not(miri))]
        assert_eq!(
            unsafe { std::ptr::addr_of!((*child).esize).read() },
            FREE_POISON_ESIZE,
            "the refused row is freed at the store"
        );
        assert_eq!(
            unsafe { alm_tbl_len(parent) },
            0,
            "the refuse path must not grow #t"
        );
        unsafe { alm_tbl_free(parent) };
    }

    #[test]
    fn refused_row_store_reclaims_without_the_header_bit() {
        // The row symbol carries the ownership proof, not the
        // destination's contains_tables bit: a row-housing parent the
        // header never flagged still reclaims its refused child.
        let parent = unsafe { alm_tbl_new(8, 0) };
        let child = unsafe { alm_tbl_new(8, 0) };

        let slot: *const u8 = std::ptr::addr_of!(child).cast();
        unsafe { alm_tbl_set_row(parent, -1, slot) };

        // The poison read: real-allocator law (see the twin above).
        #[cfg(not(miri))]
        assert_eq!(
            unsafe { std::ptr::addr_of!((*child).esize).read() },
            FREE_POISON_ESIZE,
            "the refused row is freed with no header bit to consult"
        );
        unsafe { alm_tbl_free(parent) };
    }

    #[test]
    fn refused_handle_store_drops_even_when_the_bit_is_set() {
        // The twin face law: a pooled handle (an interned string or a
        // closure's creation env) is never owned by the destination,
        // so the handle symbol drops a refused store even into a
        // bit-set parent -- the old bit-consulted reclaim could have
        // freed a borrowed env as a table.
        let parent = unsafe { alm_tbl_new(8, 0x80) };
        let child = unsafe { alm_tbl_new(8, 0) };

        let slot: *const u8 = std::ptr::addr_of!(child).cast();
        unsafe { alm_tbl_set(parent, -1, slot) };

        assert_ne!(
            unsafe { std::ptr::addr_of!((*child).esize).read() },
            FREE_POISON_ESIZE,
            "the refused handle is dropped, never freed as a table"
        );
        assert_eq!(
            unsafe { alm_tbl_len(parent) },
            0,
            "the refuse path must not grow #t"
        );
        unsafe { alm_tbl_free(parent) };
        unsafe { alm_tbl_free(child) };
    }

    #[test]
    fn replaced_row_store_reclaims_the_displaced_child() {
        // The replace law: a slot that already houses a table stores a
        // second one, so the displaced occupant is left with zero owners
        // the moment its last pointer is clobbered — set_core frees it
        // before the write, and the replacement rides the slot after.
        let parent = unsafe { alm_tbl_new(8, 0x80) };
        let first = unsafe { alm_tbl_new(8, 0) };
        let second = unsafe { alm_tbl_new(8, 0) };

        let a: *const u8 = std::ptr::addr_of!(first).cast();
        unsafe { alm_tbl_set_row(parent, 0, a) };
        let b: *const u8 = std::ptr::addr_of!(second).cast();
        unsafe { alm_tbl_set_row(parent, 0, b) };

        // The poison read on the displaced (freed) header: real-allocator
        // law (see refused_row_store_reclaims_the_child).
        #[cfg(not(miri))]
        assert_eq!(
            unsafe { std::ptr::addr_of!((*first).esize).read() },
            FREE_POISON_ESIZE,
            "the displaced row is freed at the store that displaces it"
        );
        assert_ne!(
            unsafe { std::ptr::addr_of!((*second).esize).read() },
            FREE_POISON_ESIZE,
            "the replacement rides the slot untouched"
        );
        assert_eq!(
            unsafe { alm_tbl_len(parent) },
            1,
            "a replace holds the border, never grows it"
        );
        // The slot's owner is the replacement now; the parent's exit
        // free walks its valid slot words and collects it.
        unsafe { alm_tbl_free(parent) };
    }

    #[test]
    fn overflow_parked_row_is_reclaimed_when_dense_grows_into_it() {
        // The park seam (probes/p22_overflow_park_dense_grow_leak.lua):
        // a store above `len + SPARSE_THRESHOLD` rides Branch 2 and parks
        // the row in the overflow map (which IS tbl.sparse_map --
        // overflow_map lazily allocates it) while tbl.mode stays Dense.
        // If tbl.len later grows so the SAME index flips into Branch 3's
        // dense envelope, the span word reads 0 (a parked index has never
        // had a span write) and the map entry drops. The bare remove used
        // to orphan the parked row behind the write's back; set_core now
        // feeds the removed lane through reclaim_displaced_child, the
        // same law the span word already obeys.
        let parent = unsafe { alm_tbl_new(8, 0x80) };
        let parked = unsafe { alm_tbl_new(8, 0) };
        let filler = unsafe { alm_tbl_new(8, 0) };
        let replacement = unsafe { alm_tbl_new(8, 0) };

        // Park: 100001 > 0 + SPARSE_THRESHOLD. The map allocates, the
        // border stretches to the parked index + 1, and the span stays
        // empty -- the park must never be a span write.
        let parked_lane: *const u8 = std::ptr::addr_of!(parked).cast();
        unsafe { alm_tbl_set_row(parent, 100_001, parked_lane) };
        assert_eq!(
            unsafe { (*parent).len },
            0,
            "the overflow park must not grow the span"
        );
        assert_eq!(unsafe { alm_tbl_len(parent) }, 100_002);
        assert!(!unsafe { (*parent).sparse_map }.is_null());
        assert_eq!(
            unsafe { (&*(*parent).sparse_map).get(&100_001).copied() },
            Some(parked as usize as u128),
            "the row's lane lives in the overflow map"
        );

        // Grow the dense span past the seam: index 9 (<= 10 +
        // SPARSE_THRESHOLD) spands to len 10, so the next write to
        // 100001 (<= 10 + SPARSE_THRESHOLD) takes Branch 3.
        let filler_lane: *const u8 = std::ptr::addr_of!(filler).cast();
        unsafe { alm_tbl_set_row(parent, 9, filler_lane) };
        assert_eq!(unsafe { (*parent).len }, 10);
        assert_eq!(
            unsafe { (&*(*parent).sparse_map).get(&100_001).copied() },
            Some(parked as usize as u128),
            "the unrelated grow keeps the park parked"
        );

        // Branch 3 over the parked index: the span word was 0, the map
        // entry drops, and -- the fix -- the parked row frees before the
        // replacement takes the slot.
        let replacement_lane: *const u8 = std::ptr::addr_of!(replacement).cast();
        unsafe { alm_tbl_set_row(parent, 100_001, replacement_lane) };

        // The reclaim's poison read: real-allocator law (see
        // replaced_row_store_reclaims_the_displaced_child). Under Miri
        // the freed header is exhausted, and the zero-leak exit (the
        // interpreter's own detector) carries the proof: the parked row
        // is nobody's child anymore, so only the reclaim could have
        // collected it.
        #[cfg(not(miri))]
        assert_eq!(
            unsafe { std::ptr::addr_of!((*parked).esize).read() },
            FREE_POISON_ESIZE,
            "the parked row frees at the dense flip, not at the drop"
        );
        assert!(unsafe { (&*(*parent).sparse_map).get(&100_001) }.is_none());
        assert_eq!(unsafe { (*parent).len }, 100_002);
        let lane = unsafe { ((*parent).data as *const u64).add(100_001).read() };
        assert_eq!(
            lane as usize, replacement as usize,
            "the replacement's lane is the span slot's answer"
        );
        unsafe {
            let mut got: u64 = 0;
            alm_tbl_get(parent, 100_001, ptr::addr_of_mut!(got).cast(), 8);
            assert_eq!(
                got as usize, replacement as usize,
                "the read face answers the replacement through the span"
            );
        };
        assert_ne!(
            unsafe { std::ptr::addr_of!((*replacement).esize).read() },
            FREE_POISON_ESIZE,
            "the replacement rides the slot untouched"
        );

        // The span's rows collect at the exit: the filler and the
        // replacement are the parent's valid slot words now.
        unsafe { alm_tbl_free(parent) };
    }

    #[test]
    fn row_store_self_replace_reclaims_nothing() {
        // Resetting a slot to its own row is not a replace: the old and
        // the new are the same table, and freeing the lane the incoming
        // store is about to house would clobber a live owner.
        let parent = unsafe { alm_tbl_new(8, 0x80) };
        let child = unsafe { alm_tbl_new(8, 0) };

        let slot: *const u8 = std::ptr::addr_of!(child).cast();
        unsafe { alm_tbl_set_row(parent, 0, slot) };
        let border = unsafe { alm_tbl_len(parent) };
        unsafe { alm_tbl_set_row(parent, 0, slot) };

        assert_ne!(
            unsafe { std::ptr::addr_of!((*child).esize).read() },
            FREE_POISON_ESIZE,
            "a self-replace holds the child, never frees it"
        );
        assert_eq!(
            unsafe { alm_tbl_len(parent) },
            border,
            "a self-replace rewrites one slot, nothing else"
        );
        unsafe { alm_tbl_free(parent) };
    }

    #[test]
    fn handle_replace_drops_the_displaced_row_as_before() {
        // The non-owned faces keep their recorded no-op-store law: a
        // pooled handle (interned string, closure env) is never owned
        // by the destination, so its replace drops the old cell without
        // the free — freeing could reach a borrowed env. The orphaning
        // that law implies for row slots is the static Row/Handle
        // split's problem to never mint, not the runtime's to guess.
        let parent = unsafe { alm_tbl_new(8, 0x80) };
        let child = unsafe { alm_tbl_new(8, 0) };

        let row: *const u8 = std::ptr::addr_of!(child).cast();
        unsafe { alm_tbl_set_row(parent, 0, row) };
        let word: u64 = 5;
        unsafe { alm_tbl_set(parent, 0, ptr::addr_of!(word).cast()) };

        assert_ne!(
            unsafe { std::ptr::addr_of!((*child).esize).read() },
            FREE_POISON_ESIZE,
            "the handle replace drops, never frees, the displaced row"
        );
        let word_slot = unsafe { (*parent).data } as *const u64;
        assert_eq!(
            unsafe { word_slot.read() },
            5,
            "the scalar landed in the displaced slot"
        );
        // The orphaned child is this test's own; free it directly. The
        // 5-word slot fails valid_slot_addr, so the parent's exit walk
        // skips it instead of freeing the scalar as a table.
        unsafe { alm_tbl_free(child) };
        unsafe { alm_tbl_free(parent) };
    }

    #[test]
    fn negative_keys_never_pollute_border() {
        for flags in [0u8, 1u8] {
            let t = unsafe { alm_tbl_new(std::mem::size_of::<i128>(), flags) };
            let key = str_key_of_bytes(b"health");

            unsafe { alm_tbl_set_any(t, 0, 41) };
            unsafe { alm_tbl_set_any(t, 1, 7) };
            unsafe { alm_tbl_set_any_skey(t, key, alm_any_pack(3, 0x1234)) };

            assert_eq!(
                unsafe { alm_tbl_len(t) },
                2,
                "record write must not touch #t"
            );
            assert_eq!(
                unsafe { alm_tbl_get_any_skey(t, key) },
                alm_any_pack(3, 0x1234)
            );
            assert_eq!(unsafe { alm_tbl_get_any(t, 0) }, 41);
            assert_eq!(unsafe { alm_tbl_get_any(t, 1) }, 7);
            assert_eq!(
                unsafe { alm_tbl_get_any_skey(t, str_key_of_bytes(b"missing")) },
                0
            );
            unsafe { alm_tbl_free(t) };
        }
    }

    #[test]
    fn str_range_integers_drop_through_the_int_faces() {
        let t = unsafe { alm_tbl_new(std::mem::size_of::<i128>(), 0) };
        let forged = str_key_of_bytes(b"health");
        assert!(is_str_key(forged));

        unsafe { alm_tbl_set_any(t, forged, alm_any_pack(3, 0x1234)) };
        assert_eq!(
            unsafe { alm_tbl_len(t) },
            0,
            "a forged str-range int must not grow #t"
        );
        assert_eq!(
            unsafe { alm_tbl_get_any_skey(t, str_key_of_bytes(b"health")) },
            0,
            "a forged str-range int must not create the record cell"
        );
        assert_eq!(
            unsafe { alm_tbl_get_any(t, forged) },
            0,
            "a forged str-range int reads the zero lane, not a cell"
        );
        unsafe { alm_tbl_free(t) };
    }

    #[test]
    fn interned_strings_carry_their_key_in_the_prefix() {
        let a = unsafe { alm_str_intern(b"player".as_ptr(), 6) };
        let b = unsafe { alm_str_intern(b"player".as_ptr(), 6) };
        assert_eq!(a, b, "intern dedup within the process pool");

        let key = unsafe { alm_str_key(a) };
        assert!(is_str_key(key), "pooled strings carry negative str keys");
        assert_eq!(key, str_key_of_bytes(b"player"), "prefix cache == derive");

        assert_eq!(unsafe { alm_str_len(a) }, 6);
        assert_eq!(unsafe { *a }, b'p');
    }

    #[test]
    fn key_derive_is_stable_and_disjoint_from_program_integers() {
        let k1 = str_key_of_bytes(b"health");
        let k2 = str_key_of_bytes(b"health");
        assert_eq!(k1, k2);
        assert_ne!(str_key_of_bytes(b"health"), str_key_of_bytes(b"wealth"));
        assert!(k1 < 0);
        assert!(is_str_key(k1));
        assert!(!is_str_key(-1) && !is_str_key(-42) && !is_str_key(5));
    }

    #[test]
    fn null_handle_keys_as_the_empty_string() {
        let empty = str_key_of_bytes(b"");
        assert_eq!(unsafe { alm_str_key(std::ptr::null()) }, empty);
        assert!(is_str_key(empty));
        let t = unsafe { alm_tbl_new(std::mem::size_of::<i128>(), 0) };
        unsafe { alm_tbl_set_any_skey(t, alm_str_key(std::ptr::null()), alm_any_pack(3, 0x1234)) };
        assert_eq!(
            unsafe { alm_tbl_get_any_skey(t, empty) },
            alm_any_pack(3, 0x1234),
            "a store through the null handle lands in t[\"\"], the empty string's own slot"
        );
        // The int face keeps its own law: a negative index is refused
        // (alm_str_key's null answer is a tagged key, never -1 now).
        unsafe { alm_tbl_set_any(t, -1, alm_any_pack(3, 0x1234)) };
        assert_eq!(
            unsafe { alm_tbl_len(t) },
            0,
            "a store through a negative index must no-op"
        );
        assert_eq!(
            unsafe { alm_tbl_get_any(t, 0) },
            0,
            "array slot 0 must stay untouched (answering 0 would alias it)"
        );
        unsafe { alm_tbl_free(t) };
    }

    #[test]
    fn the_empty_string_interns_as_the_null_handle() {
        // The one-empty law, at every producing face: zero-length
        // interning answers the null handle — the Rust export, the C
        // pool beneath it, concat's product — so the literal "" and a
        // missing cell's zero lane are one value. Pointer equality is
        // all `==` ever does, and one value needs no normalization.
        assert_eq!(unsafe { alm_str_intern(b"".as_ptr(), 0) }, ptr::null());
        assert_eq!(
            unsafe { alm_str_concat(ptr::null(), ptr::null()) },
            ptr::null(),
            "concat of two empties is the one empty"
        );
        assert_eq!(
            unsafe { alm_str_key(ptr::null()) },
            EMPTY_STR_KEY,
            "the one empty keys as the empty string's own minted key"
        );
    }
}

#[cfg(test)]
mod free_except_tests {
    use super::*;

    // The keep faces behind every returned-table borrow: free_tbl
    // walks the parent's valid slot words and the keep set is the
    // exact-match veto at each recursion step. These seams carried
    // zero unit tests until W2 — the closure-over-rows corpus case
    // exercises them end to end; these pin the veto law at the seam.

    #[test]
    fn free_except_spares_the_kept_row_and_frees_the_rest() {
        let parent = unsafe { alm_tbl_new(8, 0x80) };
        let keep = unsafe { alm_tbl_new(8, 0) };
        let dropme = unsafe { alm_tbl_new(8, 0) };

        unsafe { alm_tbl_set_row(parent, 0, std::ptr::addr_of!(keep).cast()) };
        unsafe { alm_tbl_set_row(parent, 1, std::ptr::addr_of!(dropme).cast()) };
        assert_eq!(unsafe { alm_tbl_len(parent) }, 2);

        unsafe { alm_tbl_free_except(parent, keep) };

        // The unkept sibling died at the walk (real-allocator law; the
        // Miri stance is the strkey_tests poison note).
        #[cfg(not(miri))]
        assert_eq!(
            unsafe { std::ptr::addr_of!((*dropme).esize).read() },
            FREE_POISON_ESIZE,
            "the unkept row is reclaimed by the parent's walk"
        );
        // The kept row is alive and usable, then freeable clean — the
        // keep was a skip, not a mark: validate_header_before_free
        // must still pass on it.
        let word: u64 = 7;
        unsafe { alm_tbl_set(keep, 0, ptr::addr_of!(word).cast()) };
        let mut back: u64 = 0;
        unsafe { alm_tbl_get(keep, 0, ptr::addr_of_mut!(back).cast(), 8) };
        assert_eq!(back, 7, "the kept row survived the parent's free");
        unsafe { alm_tbl_free(keep) };
    }

    #[test]
    fn free_except_n_spares_every_kept_row() {
        let parent = unsafe { alm_tbl_new(8, 0x80) };
        let a = unsafe { alm_tbl_new(8, 0) };
        let b = unsafe { alm_tbl_new(8, 0) };
        let c = unsafe { alm_tbl_new(8, 0) };

        unsafe { alm_tbl_set_row(parent, 0, std::ptr::addr_of!(a).cast()) };
        unsafe { alm_tbl_set_row(parent, 1, std::ptr::addr_of!(b).cast()) };
        unsafe { alm_tbl_set_row(parent, 2, std::ptr::addr_of!(c).cast()) };

        let keeps = [a, c];
        unsafe { alm_tbl_free_except_n(parent, keeps.as_ptr(), keeps.len()) };

        #[cfg(not(miri))]
        assert_eq!(
            unsafe { std::ptr::addr_of!((*b).esize).read() },
            FREE_POISON_ESIZE,
            "only the listed rows are spared"
        );
        for (i, t) in [a, c].into_iter().enumerate() {
            let word = 10 + i as u64;
            unsafe { alm_tbl_set(t, 0, ptr::addr_of!(word).cast()) };
            let mut back: u64 = 0;
            unsafe { alm_tbl_get(t, 0, ptr::addr_of_mut!(back).cast(), 8) };
            assert_eq!(back, word, "kept row {i} survived the parent's free");
            unsafe { alm_tbl_free(t) };
        }
    }

    #[test]
    fn free_except_n_null_keeps_frees_everything() {
        // The null slice is the plain-free arm by contract: no veto,
        // every valid slot word reclaimed.
        let parent = unsafe { alm_tbl_new(8, 0x80) };
        let child = unsafe { alm_tbl_new(8, 0) };
        unsafe { alm_tbl_set_row(parent, 0, std::ptr::addr_of!(child).cast()) };

        unsafe { alm_tbl_free_except_n(parent, std::ptr::null(), 0) };

        #[cfg(not(miri))]
        assert_eq!(
            unsafe { std::ptr::addr_of!((*child).esize).read() },
            FREE_POISON_ESIZE,
            "a null keep list reclaims the whole subtree"
        );
    }
}
