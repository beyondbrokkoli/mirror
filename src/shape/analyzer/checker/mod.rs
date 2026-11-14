use super::super::core::{BOUNDARY_ROOT, MOVED_ROOT, TableShape};
use super::super::facts::{
    BuiltinVerb, FfiMint, FfiVerb, FnDef, InstanceId, InstanceInfo, KeyLane,
};
use super::super::{is_ghost, is_root, join_ty};
use super::WalkState;
use super::constraints::UnknownStore;
use super::{calls_through_fn_var_msg, dotted_callee, value_call_msg};
use crate::ast::{BinOp, CtorKey, Expr, NodeId, Stmt, UnOp};
use crate::diag::Diagnostic;
use crate::shape::helpers::const_key_value;
use crate::shape::ty::Ty;
use alm_rt::{signal, trace};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU32, Ordering};

fn mint_instance() -> InstanceId {
    static NEXT: AtomicU32 = AtomicU32::new(1);
    InstanceId(NEXT.fetch_add(1, Ordering::Relaxed))
}

fn snap_of(walk: &WalkState, severed: &BTreeMap<String, BTreeSet<usize>>) -> WalkSnap {
    // D3: WalkSnap stays flat this phase — the depth-0 bindings
    // project back to the three flat faces the checker's read sites
    // consume (binds/scalars/fns). Fusing the checker's own snapshot
    // is a separate session.
    let mut binds = BTreeMap::new();
    let mut scalars = BTreeMap::new();
    let mut fns = BTreeMap::new();
    for (name, bind) in &walk.scopes[0] {
        binds.insert(name.clone(), bind.shape.clone());
        if let Some(id) = bind.scalar_node {
            scalars.insert(name.clone(), id);
        }
        fns.insert(name.clone(), bind.fn_ptr);
    }
    WalkSnap {
        binds,
        scalars,
        fns,
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

/// The cdef grammar's scalar tier: kinds that map onto the existing
/// esize lanes. `name`/`elem`/`capacity`/`reject-surface` are the
/// registry row's projections.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ScalarElem {
    Int,
    Flt,
    Bool,
}

impl ScalarElem {
    /// The one reader that maps a cdef scalar onto the checker's Ty
    /// lanes — every consumer projects through here.
    fn ty(self) -> Ty {
        match self {
            ScalarElem::Int => Ty::Int,
            ScalarElem::Flt => Ty::Flt,
            ScalarElem::Bool => Ty::Bool,
        }
    }

    /// esize in bytes, per the runtime's esize lanes ({1,8,16}).
    fn esize(self) -> i64 {
        match self {
            ScalarElem::Int | ScalarElem::Flt => 8,
            ScalarElem::Bool => 1,
        }
    }
}

/// One `ffi.cdef` registry row. `elem: None` is a struct/union name —
/// registered so later milestones can hang typed rows off it, but
/// `ffi.new` over it rejects by name.
#[derive(Clone, Debug)]
pub(super) struct CdefRow {
    pub elem: Option<ScalarElem>,
    pub capacity: Option<i64>,
}

#[derive(Clone)]
pub(super) struct WalkSnap {
    binds: BTreeMap<String, TableShape>,
    scalars: BTreeMap<String, usize>,
    fns: BTreeMap<String, Option<NodeId>>,
    severed: BTreeMap<String, BTreeSet<usize>>,
}

pub(super) struct CheckCtx {
    pub index_base: i64,
    pub adopt_lines: BTreeMap<String, usize>,
    pub sites: BTreeMap<NodeId, usize>,
    pub elems: Vec<Ty>,
    pub fn_def_sites: BTreeMap<NodeId, BTreeMap<String, Option<NodeId>>>,
    pub fn_captures: BTreeMap<NodeId, BTreeSet<String>>,
    pub fn_capture_depths: BTreeMap<NodeId, BTreeMap<String, usize>>,
}

pub(super) struct CheckFacts {
    pub local_types: BTreeMap<(NodeId, InstanceId, String), Ty>,
    pub fn_defs: BTreeMap<NodeId, FnDef>,
    pub call_fns: BTreeMap<NodeId, BTreeSet<NodeId>>,
    pub env_faces: BTreeMap<NodeId, BTreeMap<String, Ty>>,
    pub env_face_depths: BTreeMap<NodeId, BTreeMap<String, usize>>,
    pub closure_fields: BTreeMap<(usize, String), NodeId>,
    pub ffi_verbs: BTreeMap<NodeId, FfiVerb>,
    pub builtin_verbs: BTreeMap<NodeId, BuiltinVerb>,
    pub ffi_mints: BTreeMap<NodeId, FfiMint>,
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
    env_face_depths: BTreeMap<NodeId, BTreeMap<String, usize>>,
    closure_fields: BTreeMap<(usize, String), NodeId>,
    expansion_envs: Vec<Option<BTreeMap<String, Ty>>>,
    env_overrides: Vec<BTreeMap<String, Ty>>,
    cond_depth: usize,
    ffi_verbs: BTreeMap<NodeId, FfiVerb>,
    builtin_verbs: BTreeMap<NodeId, BuiltinVerb>,
    ffi_mints: BTreeMap<NodeId, FfiMint>,
    /// The compile-time cdef declaration registry — `name -> row`.
    /// Lives and dies with the checker; nothing about it reaches the
    /// runtime or the plate.
    cdef_registry: BTreeMap<String, CdefRow>,
    /// The declared capacity of the name's current mint binding, for
    /// the static-key bounds check. Maintained at the local-decl and
    /// assignment choke points.
    mint_caps: BTreeMap<String, i64>,
    /// The compile's index base, for physical-key comparison at the
    /// mint bounds check.
    index_base: i64,
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
