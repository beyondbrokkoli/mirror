mod bridges;

use crate::ast::{BinOp, CtorKey, Expr, NodeId, Stmt, UnOp};
use crate::diag::Diagnostic;
use crate::ir::{
    AnyReg, BasicBlock, BlockId, Bool, CellRepr, DbgBind, Instruction, Int, IrProgram, Ptr, Reg,
    RegId, RegKind, SourceLoc, TableKey, Terminator, repr_of,
};
use crate::shape::{
    ArmFreeEntry, BuiltinVerb, DoExitFree, FfiMint, FfiVerb, InstanceId, Keep, KeyLane,
    LayoutVerdict, ReservePlan, ShapeFacts, TagSrc, Ty,
};
use alm_rt::{signal, trace};
use bridges::{
    NumPair, NumSingle, OrdPair, any_of, bool_of, int_of, num_pair, num_single, ord_pair, phi_of,
    phi_push, pooled_ptr_of, ptr_of, str_lane, string_repr,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub struct LowerError(pub Diagnostic);

pub fn boundary_elem_of(elem: Ty) -> alm_rt::AlmElem {
    match elem {
        Ty::Flt => alm_rt::AlmElem::Float,
        Ty::Bool => alm_rt::AlmElem::Boolean,
        Ty::Str => alm_rt::AlmElem::String,
        Ty::Any => alm_rt::AlmElem::Any,
        Ty::Int | Ty::Tbl(_) | Ty::Unknown(_) => alm_rt::AlmElem::Integer,
        Ty::Ffi | Ty::FfiLib | Ty::Async => alm_rt::AlmElem::Integer,
        Ty::Closure(_) => unreachable!("a closure never rides the boundary elem"),
        Ty::Pending | Ty::Conflict => unreachable!("flow state never rides the artifact path"),
    }
}

fn join_phi_ty(a: &Ty, b: &Ty) -> Ty {
    if a == b {
        return a.clone();
    }
    let heap = |t: &Ty| matches!(t, Ty::Tbl(_));
    if heap(a) { a.clone() } else { b.clone() }
}

enum ReturnKind {
    Boundary,
    Inline { ty: Ty },
}

struct ReturnCtx {
    kind: ReturnKind,
    exit: Option<BlockId>,
    args: Vec<(BlockId, AnyReg)>,
}

#[derive(Clone)]
struct Local {
    reg: AnyReg,
    /// A CAPTURED binding's ONE cell, minted at declaration — the
    /// judge's box-at-declaration model. `Some` means the cell IS the
    /// binding's only storage: reads load `cell[0]`, writes store, and
    /// `reg` is merely the box pointer (never a value). `None` is a
    /// plain register binding — the fast path, zero overhead.
    cell: Option<Reg<Ptr>>,
    /// The TABLE capture view's redirect (scalars are cell-backed and
    /// never carry it): a direct-surface view of a depth-0 table
    /// capture re-points reads and writes at the owner slot — tables
    /// alias by pointer, no box rides a `Tbl` face, so the view has no
    /// cell of its own and the owner's register is the storage.
    capture: Option<usize>,
    ty: Ty,
    layout: LayoutVerdict,
}

#[derive(Clone)]
struct TypedReg {
    reg: AnyReg,
    ty: Ty,
}

struct BreakCtx {
    exit_block: BlockId,
    phi_vars: Vec<String>,
    breaks: Vec<(BlockId, Vec<Local>)>,
}

struct FreeHandles {
    site_regs: BTreeMap<usize, (Reg<Ptr>, u32)>,
}

/// One active conversion loop's bound register, pushed at the loop's
/// header and popped at its body's end. The eligibility and reserve
/// decisions live in the walk facts, read by NodeId; only the lowered
/// bound register is emission state. The offset register pairs with
/// the bound at the runtime's checked sum (the widening seam); row
/// and ctor reserves borrow both.
struct ActiveReserve {
    loop_id: NodeId,
    bound: Reg<Int>,
    offset: Option<Reg<Int>>,
}

/// The boxed bindings' drain inventory, keyed by the cell's register
/// id: every env table that holds the cell (the drain plan's holders,
/// in mint order). Cell-keyed, not (slot, name)-keyed — two declares
/// of one name in a scope are two bindings, two cells, two entries.
type BindingCells = BTreeMap<RegId, Vec<Reg<Ptr>>>;

pub struct IrLowerer<'a> {
    pub blocks: Vec<BasicBlock>,
    pub diagnostics: Vec<Diagnostic>,
    current_block: BlockId,
    free_reg: RegId,
    scopes: Vec<BTreeMap<String, Local>>,
    reserves: Vec<ActiveReserve>,
    handles: FreeHandles,
    shape: &'a ShapeFacts,
    cur_loc: Option<SourceLoc>,
    ghost_keeps: BTreeMap<usize, Reg<Ptr>>,
    if_tag_regs: BTreeMap<NodeId, Reg<Bool>>,
    call_tag_regs: BTreeMap<NodeId, Reg<Bool>>,

    ctrl_depth: u32,
    return_ctxs: Vec<ReturnCtx>,
    break_ctxs: Vec<BreakCtx>,
    instance_chain: Vec<InstanceId>,
    event_cursor: BTreeMap<(InstanceId, NodeId), usize>,
    env_memos: Vec<EnvMemo>,
    env_regs: Vec<Reg<Ptr>>,
    /// THE one box cell per captured binding, minted at its
    /// declaration — the judge's one-box model: "boxes once at
    /// declaration, every access goes through the one cell". Every env
    /// minted over the binding stores THIS cell's pointer (shared by
    /// Handle — no per-mint snapshot), so a write through any env
    /// reaches every view and the definer alike. The env lists are the
    /// drain plan: at root exit each holder keeps the cell until the
    /// LAST holder's `TableFreeExcept` frees it — the runtime's child
    /// walk has no refcount, so the keeps mechanism is the
    /// single-ownership proof.
    binding_cells: BindingCells,
    /// The expansion frame stack: (body scope index, body walk depth)
    /// per fn body currently lowering. The declare site's walk depth
    /// — the box-on-declare gate's key — is the frame's depth plus the
    /// scope offset within the body; the root frame is implicit
    /// (index 0, depth 0).
    fn_frames: Vec<(usize, usize)>,
    /// The captured bindings' identities as (walk-scope depth, name):
    /// the union of every fn literal's capture map. A declare whose
    /// identity is here mints its box at declaration.
    captured: BTreeSet<(usize, String)>,
    /// Per scope, the cells its OWN declares minted (index-aligned
    /// with `scopes`). Capture views hold borrowed cells — another
    /// scope's — and never appear here, so a dying scope frees only
    /// what it owns.
    scope_cells: Vec<BTreeSet<RegId>>,
    cur_stmt: Option<NodeId>,
}

/// One statement's minted env. `depths` is the binding identity of
/// every cell the env holds: a capture's walk-scope index, not its
/// string. A later creation read in the same statement shares this
/// env only when every name the two faces agree on resolves to the
/// same depth — same name, different depth is a different binding
/// (the shadow), and it mints its own env (Closure Finding A).
struct EnvMemo {
    stmt: NodeId,
    reg: Reg<Ptr>,
    stored: BTreeSet<String>,
    depths: BTreeMap<String, usize>,
}

include!("impl_core.rs");
include!("impl_program.rs");
include!("impl_fact_reads.rs");
include!("impl_stmt.rs");
include!("impl_layout_ffi.rs");
include!("impl_env.rs");
include!("impl_expr.rs");
include!("collectors.rs");
include!("break_gate_tests.rs");
