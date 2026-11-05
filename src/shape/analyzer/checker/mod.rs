use super::super::core::{BOUNDARY_ROOT, MOVED_ROOT, TableShape};
use super::super::facts::{FfiVerb, FnDef, InstanceId, InstanceInfo, KeyLane};
use super::super::{is_ghost, is_root, join_ty};
use super::WalkState;
use super::constraints::UnknownStore;
use super::{calls_through_fn_var_msg, dotted_callee, value_call_msg};
use crate::ast::{BinOp, CtorKey, Expr, NodeId, Stmt, UnOp};
use crate::diag::Diagnostic;
use crate::shape::ty::Ty;
use alm_rt::{signal, trace};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU32, Ordering};

fn mint_instance() -> InstanceId {
    static NEXT: AtomicU32 = AtomicU32::new(1);
    InstanceId(NEXT.fetch_add(1, Ordering::Relaxed))
}

fn snap_of(walk: &WalkState, severed: &BTreeMap<String, BTreeSet<usize>>) -> WalkSnap {
    WalkSnap {
        binds: walk.scopes[0].clone(),
        scalars: walk.scalar_nodes[0].clone(),
        fns: walk.fn_scopes[0].clone(),
        severed: severed
            .iter()
            .filter(|(name, _)| walk.scopes[0].contains_key(*name))
            .map(|(name, sites)| (name.clone(), sites.clone()))
            .collect(),
    }
}

#[derive(Clone)]
enum ReturnCtx {
    Boundary,
    Internal(Ty),
}

struct Frame {
    types: BTreeMap<String, Ty>,
    fns: BTreeMap<String, Option<NodeId>>,
}

#[derive(Clone)]
pub(super) struct WalkSnap {
    binds: BTreeMap<String, TableShape>,
    scalars: BTreeMap<String, usize>,
    fns: BTreeMap<String, Option<NodeId>>,
    severed: BTreeMap<String, BTreeSet<usize>>,
}

pub(super) struct CheckCtx {
    pub adopt_lines: BTreeMap<String, usize>,
    pub sites: BTreeMap<NodeId, usize>,
    pub elems: Vec<Ty>,
    pub fn_def_sites: BTreeMap<NodeId, BTreeMap<String, Option<NodeId>>>,
    pub fn_captures: BTreeMap<NodeId, BTreeSet<String>>,
}

pub(super) struct CheckFacts {
    pub local_types: BTreeMap<(NodeId, InstanceId, String), Ty>,
    pub fn_defs: BTreeMap<NodeId, FnDef>,
    pub call_fns: BTreeMap<NodeId, BTreeSet<NodeId>>,
    pub env_faces: BTreeMap<NodeId, BTreeMap<String, Ty>>,
    pub closure_fields: BTreeMap<(usize, String), NodeId>,
    pub ffi_verbs: BTreeMap<NodeId, FfiVerb>,
    pub instances: BTreeMap<InstanceId, InstanceInfo>,
    pub boundary_elem: Option<Ty>,
    pub diagnostics: Vec<Diagnostic>,
}

pub(super) struct Checker {
    frames: Vec<Frame>,
    top: BTreeMap<String, Ty>,
    cur_decls: BTreeSet<String>,
    snap: WalkSnap,
    decl_snap: WalkSnap,
    ctx: CheckCtx,
    return_ctxs: Vec<ReturnCtx>,
    inline_stack: Vec<NodeId>,
    expansion_frame_bases: Vec<usize>,
    expansion_params: Vec<Vec<String>>,
    instance_stack: Vec<InstanceId>,
    expansion_break_bases: Vec<usize>,
    seed_read: bool,
    len_deferrals: Vec<(Ty, Option<usize>)>,
    cur_stmt: Option<NodeId>,
    cur_stmt_line: Option<usize>,
    loop_depth: usize,
    fn_defs: BTreeMap<NodeId, FnDef>,
    call_fns: BTreeMap<NodeId, BTreeSet<NodeId>>,
    env_faces: BTreeMap<NodeId, BTreeMap<String, Ty>>,
    closure_fields: BTreeMap<(usize, String), NodeId>,
    expansion_envs: Vec<Option<BTreeMap<String, Ty>>>,
    env_overrides: Vec<BTreeMap<String, Ty>>,
    cond_depth: usize,
    ffi_verbs: BTreeMap<NodeId, FfiVerb>,
    instances: BTreeMap<InstanceId, InstanceInfo>,
    local_types: BTreeMap<(NodeId, InstanceId, String), Ty>,
    bare_sites: BTreeMap<(NodeId, InstanceId, String), usize>,
    boundary_elem: Option<Ty>,
    diagnostics: Vec<Diagnostic>,
    stmt_position_call: bool,
}

include!("impl_core.rs");
include!("impl_scopes.rs");
include!("impl_stmt.rs");
include!("impl_fact_writers.rs");
include!("impl_ffi.rs");
include!("impl_closure_faces.rs");
include!("impl_expr.rs");
include!("expand_call.rs");
include!("impl_expr_helpers.rs");
include!("type_helpers.rs");
