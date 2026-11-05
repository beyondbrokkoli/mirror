mod bridges;

use crate::ast::{BinOp, CtorKey, Expr, NodeId, Stmt, UnOp};
use crate::diag::Diagnostic;
use crate::ir::{
    AnyReg, BasicBlock, BlockId, Bool, CellRepr, DbgBind, Instruction, Int, IrProgram, Ptr, Reg,
    RegId, RegKind, SourceLoc, TableKey, Terminator, repr_of,
};
use crate::shape::{
    ArmFreeEntry, DoExitFree, FfiVerb, InstanceId, Keep, KeyLane, LayoutVerdict, ShapeFacts,
    TagSrc, Ty,
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
        Ty::Ffi | Ty::FfiLib | Ty::Coro => alm_rt::AlmElem::Integer,
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
    ty: Ty,
    layout: LayoutVerdict,
}

#[derive(Clone)]
struct TypedReg {
    reg: AnyReg,
    ty: Ty,
}

struct LoopCtx {
    guard_reg: RegId,
    reserved: Vec<RegId>,
    nested_fills: Vec<(String, Reg<Int>)>,
}

struct BreakCtx {
    exit_block: BlockId,
    phi_vars: Vec<String>,
    breaks: Vec<(BlockId, Vec<Local>)>,
}

struct FreeHandles {
    site_regs: BTreeMap<usize, (Reg<Ptr>, u32)>,
}

struct FillCtxs {
    loop_ctxs: Vec<LoopCtx>,
}

pub struct IrLowerer<'a> {
    pub blocks: Vec<BasicBlock>,
    pub diagnostics: Vec<Diagnostic>,
    current_block: BlockId,
    free_reg: RegId,
    scopes: Vec<BTreeMap<String, Local>>,
    fills: FillCtxs,
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
    env_memo: Option<EnvMemo>,
    env_regs: Vec<Reg<Ptr>>,
    env_ctxs: Vec<EnvCtx>,
    cur_stmt: Option<NodeId>,
}

struct EnvMemo {
    stmt: NodeId,
    reg: Reg<Ptr>,
    stored: BTreeSet<String>,
}

struct EnvCtx {
    reg: Reg<Ptr>,
    faces: BTreeMap<String, Ty>,
}
impl Clone for EnvCtx {
    fn clone(&self) -> Self {
        EnvCtx {
            reg: self.reg,
            faces: self.faces.clone(),
        }
    }
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
