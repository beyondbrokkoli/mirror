
use std::ffi::{c_char, CStr};

unsafe extern "C" {
    fn vx_exec_register(name: *const c_char, flags: u32) -> i32;
    fn vx_exec_name(id: i32) -> *const c_char;
    fn vx_exec_state_get(id: i32) -> u32;
    fn vx_exec_state_set(id: i32, state: u32);
    fn vx_exec_flags_get(id: i32) -> u32;
    fn vx_exec_census_set(id: i32, live: i64, total: i64);
    fn vx_exec_count() -> i32;
    fn vx_exec_dump();
    fn vx_trace_poke_exec(exec_id: i32, slot: u8);
    fn vx_trace_poke(slot: u8);
    fn vx_trace_fired(slot: u8) -> u8;
    fn vx_trace_event_count() -> u64;
    fn vx_trace_event_get(seq: u64) -> u32;
}

fn name_of(id: i32) -> String {
    unsafe { CStr::from_ptr(vx_exec_name(id)) }
        .to_string_lossy()
        .into_owned()
}

fn main() {
    if std::env::var_os("ALM_PROBE_TENANT_EXIT_LEAK").is_some() {
        let id = unsafe { vx_exec_register(c"worker.so".as_ptr(), 0x2) };
        assert!(id >= 0);
        unsafe { vx_exec_census_set(id, 6, 10) };
        println!("probe_tenant[child]: worker.so left live at exit on purpose");
        return;
    }

    let a = unsafe { vx_exec_register(c"main.exe".as_ptr(), 0x1) };
    let b = unsafe { vx_exec_register(c"worker.so".as_ptr(), 0x2) };
    let c = unsafe { vx_exec_register(c"leaky.so".as_ptr(), 0x2) };
    assert_eq!((a, b, c), (0, 1, 2), "exec slots must allocate 0,1,2");
    assert_eq!(unsafe { vx_exec_count() }, 3);
    assert_eq!(name_of(b), "worker.so");
    assert_eq!(unsafe { vx_exec_flags_get(a) }, 0x1);
    assert_eq!(unsafe { vx_exec_state_get(a) }, 2 );

    unsafe { vx_exec_census_set(b, 6, 10) };
    unsafe { vx_exec_state_set(c, 3 ) };
    assert_eq!(unsafe { vx_exec_state_get(c) }, 3);
    assert_eq!(name_of(99), "");

    {
        use std::process::Command;
        let tmp = std::env::temp_dir()
            .join(format!("alm_probe_tenant_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("child CWD");
        let out = Command::new(std::env::current_exe().expect("self"))
            .env("ALM_PROBE_TENANT_EXIT_LEAK", "1")
            .current_dir(&tmp)
            .output()
            .expect("sacrificial child");
        assert!(
            out.status.success(),
            "a leak report never fails the exit status"
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("slot 90"), "names its slot: {err}");
        assert!(err.contains("worker.so"), "names the module: {err}");
        assert!(err.contains("== 6"), "names the live count: {err}");
        let plate = std::fs::read(tmp.join(".alm_rt_trace.bin"))
            .expect("child's plate");
        assert_eq!(plate.len(), 256);
        assert_eq!(plate[90], 1, "slot 90 sticky on the child's plate");
        assert_eq!(plate[50], 0, "no other slot fired");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    unsafe { vx_trace_poke_exec(a, 50) };
    unsafe { vx_trace_poke_exec(b, 60) };
    unsafe { vx_trace_poke(162) };
    assert_eq!(unsafe { vx_trace_fired(50) }, 1);
    assert_eq!(unsafe { vx_trace_fired(60) }, 1);
    assert_eq!(unsafe { vx_trace_fired(162) }, 1);
    assert_eq!(unsafe { vx_trace_fired(90) }, 0, "no leak poked");

    let n = unsafe { vx_trace_event_count() };
    assert_eq!(n, 3);
    assert_eq!(unsafe { vx_trace_event_get(0) }, 50);
    assert_eq!(unsafe { vx_trace_event_get(1) }, (1u32 << 8) | 60);
    assert_eq!(unsafe { vx_trace_event_get(2) }, (0xFFu32 << 8) | 162);
    assert_eq!(
        unsafe { vx_trace_event_get(3) },
        0xFFFF_FFFF,
        "unwritten event must read NONE"
    );

    unsafe extern "C" {
        fn vx_str_intern(s: *const u8, len: usize) -> *const u8;
    }

    while unsafe { vx_exec_register(c"filler.so".as_ptr(), 0x2) } >= 0 {}
    assert_eq!(unsafe { vx_exec_count() }, 8, "registry ceiling is VX_EXEC_MAX");
    assert_eq!(
        unsafe { vx_exec_register(c"ninth.so".as_ptr(), 0x2) },
        -1,
        "the 9th register answers the distinct -1, never a silent reuse"
    );
    let before = unsafe { vx_trace_event_count() };
    unsafe { vx_trace_poke_exec(-1, 77) };
    assert_eq!(unsafe { vx_trace_fired(77) }, 1);
    assert_eq!(
        unsafe { vx_trace_event_get(before) },
        (0xFFu32 << 8) | 77,
        "an out-of-range exec id rides the anonymous tag"
    );

    let base = unsafe { vx_trace_event_count() };
    let cap: u64 = 8192;
    for _ in 0..(cap + 9) {
        unsafe { vx_trace_poke(55) };
    }
    let n = unsafe { vx_trace_event_count() };
    assert_eq!(n, base + cap + 9);
    assert_eq!(unsafe { vx_trace_event_get(n - 1) }, (0xFFu32 << 8) | 55);
    assert_eq!(
        unsafe { vx_trace_event_get(base) },
        unsafe { vx_trace_event_get(base + cap) },
        "seq and seq+CAP must read the same wrapped cell"
    );

    let h1 = unsafe { vx_str_intern(b"probe_canonical".as_ptr(), 15) };
    let h2 = unsafe { vx_str_intern(b"probe_canonical".as_ptr(), 15) };
    assert!(!h1.is_null());
    assert_eq!(h1, h2, "intern is canonical: same bytes, same handle");
    assert!(
        unsafe { vx_str_intern(std::ptr::null(), 0) }.is_null(),
        "the NULL guard propagates NULL"
    );

    unsafe { vx_exec_dump() };
    unsafe { vx_exec_census_set(b, 0, 10) };
    println!("probe_tenant: 3 execs registered, plate + ring verified");
    println!("probe_tenant: 9th register -1 + anonymous tag, ring wrap at 8192, \
              str_pool dedup — GREEN");
    println!("probe_tenant: exit leak report pinned via child (slot 90 + named \
              module), own mirror reset — exits clean");
}
