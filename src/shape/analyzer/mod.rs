use crate::analysis::AnalysisContext;
use crate::ast::{BinOp, CtorKey, Expr, NodeId, Stmt, UnOp};
use alm_rt::{signal, trace};
use std::collections::{BTreeMap, BTreeSet};

use super::core::{
    BOUNDARY_ROOT, BOUNDS_FAIL_THRESHOLD, LayoutVerdict, MOVED_ROOT, NULL_ROOT, RowLineage,
    SPARSE_THRESHOLD, TableShape, is_ghost, is_root,
};
use super::facts;
use super::facts::{FastStoreElig, KeyShape, ReservePlan, ShapeFacts};
use super::helpers::{const_key_value, extract_guard, guard_key_shape, merge_table_scopes};
use super::ty::{Ty, arith_ty, contains_any, decided, join_ty, pinned_scalar, value_kind};
pub(super) use crate::diag::Diagnostic;
use Ty::{Any, Async, Bool, Conflict, Ffi, FfiLib, Flt, Int, Pending, Str, Tbl};

mod checker;
mod checks;
mod constraints;
mod exits;
mod fast_lane;
mod functions;
mod infer;
mod lattice;
mod pins;
mod rebind;
mod rows;
mod walk;

use infer::*;
use lattice::*;
use rebind::*;
use rows::*;

struct ShapeError(pub Diagnostic);

fn boundary_root_scope() -> BTreeMap<String, TableShape> {
    let mut scope = BTreeMap::new();
    scope.insert(
        "arg".to_string(),
        TableShape {
            ty: Tbl(Box::new(Pending)),
            layout: LayoutVerdict::default(),
            aliases: BTreeSet::from([BOUNDARY_ROOT]),
            lineage: BTreeMap::new(),
        },
    );
    scope
}

struct Ledger {
    diagnostics: Vec<Diagnostic>,
    conflict_reported: BTreeSet<usize>,
}

#[derive(Clone, Copy)]
struct Recording {
    _priv: (),
}

type Gate = Option<Recording>;

trait GateOn {
    fn on(self) -> bool;
}

impl GateOn for Option<Recording> {
    fn on(self) -> bool {
        self.is_some()
    }
}

struct WalkState {
    scopes: Vec<BTreeMap<String, TableShape>>,
    fn_scopes: Vec<BTreeMap<String, Option<NodeId>>>,
    changed: bool,
    conv_fires: BTreeSet<u8>,
    prov: Vec<BTreeMap<String, Option<Prov>>>,
    scalar_nodes: Vec<BTreeMap<String, usize>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Prov {
    If(NodeId),
    Call(NodeId),
}

impl WalkState {
    /// The stack architecture's alignment law: `scopes` is the master
    /// list every rider hangs off — `fn_scopes`, `prov`, and
    /// `scalar_nodes` are index-aligned with it, one map per depth,
    /// pushed and popped together (enter/exit_scope) and reset together
    /// (both reset sites). A drift here is the orphan-ghost's incubator:
    /// a rider written at depth d describing a binding that lives at
    /// depth d' dies with the wrong scope. The release profile keeps
    /// debug-assertions live, so every corpus leg proves this, not just
    /// debug builds.
    fn assert_parallel_stacks_aligned(&self) {
        debug_assert_eq!(
            self.scopes.len(),
            self.fn_scopes.len(),
            "the stacks drifted: fn_scopes is not index-aligned with scopes"
        );
        debug_assert_eq!(
            self.scopes.len(),
            self.prov.len(),
            "the stacks drifted: prov is not index-aligned with scopes"
        );
        debug_assert_eq!(
            self.scopes.len(),
            self.scalar_nodes.len(),
            "the stacks drifted: scalar_nodes is not index-aligned with scopes"
        );
    }

    /// The riders' content law — the assert that would have caught the
    /// Oct 17 ghost on its first pass: a rider entry at depth d
    /// describes the binding its name has at that same depth. Every
    /// rider key must be bound in `scopes[d]`, with one documented
    /// exemption: a name bound nowhere at all (the undeclared-assignment
    /// rejection writes its rider before rejecting). Run after each
    /// recorded-pass statement, once every binder has run and every
    /// scope the statement opened has closed.
    fn assert_riders_bound(&self) {
        self.assert_parallel_stacks_aligned();
        let bound_nowhere = |name: &str| self.scopes.iter().all(|s| !s.contains_key(name));
        for (depth, rider) in self.scalar_nodes.iter().enumerate() {
            for name in rider.keys() {
                debug_assert!(
                    self.scopes[depth].contains_key(name) || bound_nowhere(name),
                    "the riders broke faith with the master stack: \
                     scalar_nodes[{depth}] holds '{name}' but its binding does not live there"
                );
            }
        }
        for (depth, rider) in self.fn_scopes.iter().enumerate() {
            for name in rider.keys() {
                debug_assert!(
                    self.scopes[depth].contains_key(name) || bound_nowhere(name),
                    "the riders broke faith with the master stack: \
                     fn_scopes[{depth}] holds '{name}' but its binding does not live there"
                );
            }
        }
        for (depth, rider) in self.prov.iter().enumerate() {
            for name in rider.keys() {
                debug_assert!(
                    self.scopes[depth].contains_key(name) || bound_nowhere(name),
                    "the riders broke faith with the master stack: \
                     prov[{depth}] holds '{name}' but its binding does not live there"
                );
            }
        }
    }
}

struct LatticeState {
    sites: BTreeMap<NodeId, usize>,
    site_elem: Vec<Ty>,
    needed: Vec<bool>,
    child_sites: BTreeMap<usize, BTreeSet<usize>>,
    ghost_next: usize,
    row_ghosts: BTreeMap<NodeId, usize>,
    row_ghost_next: usize,
    ghost_mints: BTreeMap<usize, GhostMint>,
    site_slots: BTreeMap<usize, BTreeMap<i64, usize>>,
    ghost_site: BTreeMap<usize, usize>,
    housed_ghosts: BTreeSet<usize>,
    claimed_ghosts: BTreeSet<usize>,
    adopt_witness: BTreeMap<usize, usize>,
    unknowns: constraints::UnknownStore,
}

struct GhostMint {
    bases: BTreeSet<usize>,
    key: Option<i64>,
}

struct OwnershipState {
    free_sites: BTreeSet<NodeId>,
    free_keeps: BTreeMap<NodeId, Vec<super::facts::Keep>>,
    do_exit_frees: BTreeMap<(NodeId, u8), Vec<super::facts::DoExitFree>>,
    root_exit_frees: Vec<super::facts::DoExitFree>,
    stmt_temp_frees: BTreeMap<NodeId, Vec<usize>>,
    cond_temp_frees: BTreeMap<NodeId, Vec<usize>>,
    arm_temp_frees: BTreeMap<NodeId, Vec<usize>>,
    ret_path_frees: BTreeMap<NodeId, Vec<super::facts::DoExitFree>>,
    break_path_frees: BTreeMap<NodeId, Vec<super::facts::DoExitFree>>,
    ret_path_keep_frees: BTreeMap<NodeId, Vec<super::facts::DoExitFree>>,
    ret_path_keep_temps: BTreeMap<NodeId, Vec<usize>>,
    rebind_grants: BTreeMap<NodeId, BTreeMap<String, Vec<super::facts::Keep>>>,
    stored_ctor_parents: BTreeSet<usize>,
    /// Every IndexAssign whose value inferred table-typed — the same
    /// predicate the lowerer's `contains_tables` ownership bit keys on.
    /// The fast lane reads it so a store's row kind survives a
    /// non-ctor spelling (`t[i] = r`), which the lane's syntactic
    /// `TableCtor` check alone would miss and mark nothing.
    row_value_stores: BTreeSet<NodeId>,
    move_poisons: BTreeMap<NodeId, BTreeSet<String>>,
    store_ghost_keeps: BTreeMap<NodeId, Vec<usize>>,
    ctor_ghost_keeps: BTreeMap<NodeId, Vec<(usize, usize)>>,
    arm_free_sites: BTreeMap<NodeId, Vec<super::facts::ArmFreeEntry>>,
    arm_rebind_frees: BTreeMap<NodeId, Vec<super::facts::ArmFreeEntry>>,
    arm_do_exit_frees: BTreeMap<(NodeId, u8), Vec<super::facts::ArmFreeEntry>>,
    arm_root_exit_frees: Vec<super::facts::ArmFreeEntry>,
}

struct HoldsState {
    hold_edges: BTreeMap<usize, BTreeSet<usize>>,
    cyclic_stores: BTreeMap<String, (String, usize)>,
    num_assigns: BTreeMap<String, Vec<NumExpr>>,
    nil_severed: BTreeMap<String, BTreeSet<usize>>,
    moved_severed: BTreeMap<String, BTreeSet<usize>>,
}

struct ReadSideState {
    tbl_value_stores: BTreeSet<usize>,
    nested_store_bases: BTreeMap<usize, String>,
    row_reads: BTreeMap<usize, (String, usize)>,
    row_links: BTreeMap<usize, BTreeSet<usize>>,
    user_store_sites: BTreeSet<usize>,
    boundary_fed: BTreeSet<usize>,
}

struct LayoutFactsState {
    name_dense: BTreeMap<(String, usize), bool>,
    fast_stores: BTreeMap<NodeId, super::facts::FastStoreElig>,
    reserve_plans: BTreeMap<NodeId, super::facts::ReservePlan>,
    verdicts: Vec<LayoutVerdict>,
}

/// The ctor sites a body mentions — rides the traversal engine; the site
/// read at every expression is the enter_expr hook.
struct BodySites<'a> {
    sites: &'a BTreeMap<NodeId, usize>,
    out: BTreeSet<usize>,
}

impl crate::visit::Visitor for BodySites<'_> {
    type Break = std::convert::Infallible;

    fn enter_expr(&mut self, expr: &Expr) -> Result<(), Self::Break> {
        if let Some(&id) = self.sites.get(&expr.id()) {
            self.out.insert(id);
        }
        Ok(())
    }

    // NOTE historical shape: the hand walker never descended a Field's
    // obj (a ctor inside `t[..].k` went unseen). Kept bit-for-bit — the
    // goldens lock it; revisit as its own change if ever needed.
    fn visit_field(&mut self, _obj: &Expr, _name: &str, _id: NodeId) -> Result<(), Self::Break> {
        Ok(())
    }
}

fn collect_body_sites(stmts: &[Stmt], sites: &BTreeMap<NodeId, usize>, out: &mut BTreeSet<usize>) {
    let mut v = BodySites {
        sites,
        out: std::mem::take(out),
    };
    let _ = crate::visit::walk_stmts(&mut v, stmts);
    *out = v.out;
}

/// Bases stored into by a body — rides the traversal engine; the base of
/// every IndexAssign obj is read before its children.
struct StoredBases {
    out: BTreeSet<String>,
}

impl crate::visit::Visitor for StoredBases {
    type Break = std::convert::Infallible;

    fn visit_index_assign(
        &mut self,
        obj: &Expr,
        key: &Expr,
        value: &Expr,
        _loc: crate::ast::Loc,
        _id: NodeId,
    ) -> Result<(), Self::Break> {
        if let Some(base) = stmt_base(obj) {
            self.out.insert(base);
        }
        crate::visit::walk_expr(self, obj)?;
        crate::visit::walk_expr(self, key)?;
        crate::visit::walk_expr(self, value)
    }

    // NOTE historical shape: same Field-obj blindness as the body-sites
    // scan above — kept bit-for-bit for the goldens.
    fn visit_field(&mut self, _obj: &Expr, _name: &str, _id: NodeId) -> Result<(), Self::Break> {
        Ok(())
    }
}

fn stmt_base(e: &Expr) -> Option<String> {
    // C: index-chain classifier — only an Identifier or an Index spine
    // names a base
    match e {
        Expr::Identifier { name: n, .. } => Some(n.clone()),
        Expr::Index { obj, .. } => stmt_base(obj),
        // C: every other expression form is not an lvalue base
        Expr::Integer { .. }
        | Expr::Float { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Nil { .. }
        | Expr::TableCtor { .. }
        | Expr::BinaryOp { .. }
        | Expr::UnaryOp { .. }
        | Expr::SysAllocCount { .. }
        | Expr::SysExecCount { .. }
        | Expr::FfiModule { .. }
        | Expr::AsyncModule { .. }
        | Expr::Field { .. }
        | Expr::Function { .. }
        | Expr::Call { .. } => None,
    }
}

fn collect_stored_bases(stmts: &[Stmt]) -> BTreeSet<String> {
    let mut v = StoredBases {
        out: BTreeSet::new(),
    };
    let _ = crate::visit::walk_stmts(&mut v, stmts);
    v.out
}

struct Analyzer {
    walk: WalkState,
    ledger: Ledger,
    lattice: LatticeState,
    own: OwnershipState,
    holds: HoldsState,
    reads: ReadSideState,
    layout: LayoutFactsState,
    walk_pins: bool,
    index_base: i64,
    pin_derived_exprs: BTreeSet<NodeId>,
    pin_justified_names: BTreeSet<String>,
    ctor_lines: Vec<usize>,
    fn_rets: Vec<Option<TableShape>>,
    fn_edge_frames: Vec<Vec<TableShape>>,
    fn_ret_shapes: BTreeMap<NodeId, TableShape>,
    fn_ret_edge_shapes: BTreeMap<NodeId, Vec<TableShape>>,
    join_arm_shapes: BTreeMap<(NodeId, String), (TableShape, TableShape)>,
    join_tags: BTreeMap<NodeId, bool>,
    call_tags: BTreeMap<NodeId, Vec<bool>>,
    fn_scope_keys: BTreeMap<NodeId, (NodeId, u8)>,
    fn_body_sites: BTreeSet<usize>,
    fn_params: BTreeMap<NodeId, Vec<String>>,
    fn_store_params: BTreeMap<NodeId, BTreeSet<String>>,
    fn_param_sites: BTreeSet<usize>,
    fn_moved_params: BTreeMap<NodeId, BTreeSet<String>>,
    fn_ret_params: BTreeMap<NodeId, BTreeSet<usize>>,
    fn_ret_reach: BTreeMap<NodeId, BTreeSet<usize>>,
    fn_ret_reaches: Vec<BTreeSet<usize>>,
    fn_walk: Vec<(NodeId, Vec<String>)>,
    fn_scope_entries: Vec<usize>,
    fn_def_sites: BTreeMap<NodeId, BTreeMap<String, Option<NodeId>>>,
    fn_captures: BTreeMap<NodeId, BTreeSet<String>>,
    loop_scope_entries: Vec<usize>,
}

/// The base-0 entry the property pins and corpus sweeps drive; the
/// pipeline calls `analyze_with_base` with the surface's index base.
#[cfg(test)]
pub fn analyze_with(ctx: &AnalysisContext<'_>, walk_pins: bool) -> ShapeFacts {
    analyze_with_base(ctx, walk_pins, 0)
}

pub fn analyze_with_base(
    ctx: &AnalysisContext<'_>,
    walk_pins: bool,
    index_base: i64,
) -> ShapeFacts {
    let n = ctx.sites.len();
    let mut a = Analyzer {
        walk_pins,
        index_base,
        pin_derived_exprs: BTreeSet::new(),
        pin_justified_names: BTreeSet::new(),
        walk: WalkState {
            scopes: vec![boundary_root_scope()],
            fn_scopes: vec![BTreeMap::new()],
            changed: false,
            conv_fires: BTreeSet::new(),
            prov: vec![BTreeMap::new()],
            scalar_nodes: vec![BTreeMap::new()],
        },
        ledger: Ledger {
            diagnostics: Vec::new(),
            conflict_reported: BTreeSet::new(),
        },
        lattice: LatticeState {
            sites: ctx.sites.clone(),
            site_elem: vec![Pending; n],
            needed: vec![false; n],
            child_sites: BTreeMap::new(),
            ghost_next: usize::MAX / 2,
            row_ghosts: BTreeMap::new(),
            row_ghost_next: super::core::ROW_GHOST_FLOOR + 1,
            ghost_mints: BTreeMap::new(),
            site_slots: BTreeMap::new(),
            ghost_site: BTreeMap::new(),
            housed_ghosts: BTreeSet::new(),
            claimed_ghosts: BTreeSet::new(),
            adopt_witness: BTreeMap::new(),
            unknowns: constraints::UnknownStore::new(),
        },
        own: OwnershipState {
            free_sites: BTreeSet::new(),
            free_keeps: BTreeMap::new(),
            do_exit_frees: BTreeMap::new(),
            root_exit_frees: Vec::new(),
            stmt_temp_frees: BTreeMap::new(),
            cond_temp_frees: BTreeMap::new(),
            arm_temp_frees: BTreeMap::new(),
            ret_path_frees: BTreeMap::new(),
            break_path_frees: BTreeMap::new(),
            ret_path_keep_frees: BTreeMap::new(),
            ret_path_keep_temps: BTreeMap::new(),
            rebind_grants: BTreeMap::new(),
            stored_ctor_parents: BTreeSet::new(),
            row_value_stores: BTreeSet::new(),
            move_poisons: BTreeMap::new(),
            store_ghost_keeps: BTreeMap::new(),
            ctor_ghost_keeps: BTreeMap::new(),
            arm_free_sites: BTreeMap::new(),
            arm_rebind_frees: BTreeMap::new(),
            arm_do_exit_frees: BTreeMap::new(),
            arm_root_exit_frees: Vec::new(),
        },
        holds: HoldsState {
            hold_edges: BTreeMap::new(),
            cyclic_stores: BTreeMap::new(),
            num_assigns: BTreeMap::new(),
            nil_severed: BTreeMap::new(),
            moved_severed: BTreeMap::new(),
        },
        reads: ReadSideState {
            tbl_value_stores: BTreeSet::new(),
            nested_store_bases: BTreeMap::new(),
            row_reads: BTreeMap::new(),
            row_links: BTreeMap::new(),
            user_store_sites: BTreeSet::new(),
            boundary_fed: BTreeSet::new(),
        },
        layout: LayoutFactsState {
            name_dense: BTreeMap::new(),
            fast_stores: BTreeMap::new(),
            reserve_plans: BTreeMap::new(),
            verdicts: vec![LayoutVerdict::Growing; n],
        },
        ctor_lines: ctx.ctor_lines.clone(),
        fn_rets: Vec::new(),
        fn_edge_frames: Vec::new(),
        fn_ret_shapes: BTreeMap::new(),
        fn_ret_edge_shapes: BTreeMap::new(),
        join_arm_shapes: BTreeMap::new(),
        join_tags: BTreeMap::new(),
        call_tags: BTreeMap::new(),
        fn_scope_keys: BTreeMap::new(),
        fn_body_sites: BTreeSet::new(),
        fn_params: BTreeMap::new(),
        fn_store_params: BTreeMap::new(),
        fn_param_sites: BTreeSet::new(),
        fn_moved_params: BTreeMap::new(),
        fn_ret_params: BTreeMap::new(),
        fn_ret_reach: BTreeMap::new(),
        fn_ret_reaches: Vec::new(),
        fn_walk: Vec::new(),
        fn_scope_entries: Vec::new(),
        fn_def_sites: BTreeMap::new(),
        fn_captures: BTreeMap::new(),
        loop_scope_entries: Vec::new(),
    };

    let mut prev_end: Vec<BTreeMap<String, TableShape>> = Vec::new();
    let mut prev_fn_end: Vec<BTreeMap<String, Option<NodeId>>> = Vec::new();
    let mut prev_prov_end: Vec<BTreeMap<String, Option<Prov>>> = Vec::new();
    let mut prev_nodes_end: Vec<BTreeMap<String, usize>> = Vec::new();
    let mut dbg_pass: u32 = 0;
    loop {
        dbg_pass += 1;
        crate::alm_dbgln!("[fixpoint] pass {dbg_pass}");
        a.walk.changed = false;
        a.walk.scopes = vec![boundary_root_scope()];
        a.walk.fn_scopes = vec![BTreeMap::new()];
        a.walk.prov = vec![BTreeMap::new()];
        a.walk.scalar_nodes = vec![BTreeMap::new()];
        a.walk.assert_parallel_stacks_aligned();
        a.fn_def_sites.clear();
        a.fn_captures.clear();
        a.walk_stmts(None, ctx.ast);
        // Stability must see every stack the walk maintains: comparing
        // scopes alone is blind to a rider oscillating in fn_scopes,
        // prov, or scalar_nodes — the loop would break on a master
        // state that settled while its riders never did, and the
        // recorded pass would inherit whatever face they wore that
        // pass.
        let end = a.walk.scopes.clone();
        let fn_end = a.walk.fn_scopes.clone();
        let prov_end = a.walk.prov.clone();
        let nodes_end = a.walk.scalar_nodes.clone();
        let stable = end == prev_end
            && fn_end == prev_fn_end
            && prov_end == prev_prov_end
            && nodes_end == prev_nodes_end
            && !a.walk.changed;
        prev_end = end;
        prev_fn_end = fn_end;
        prev_prov_end = prov_end;
        prev_nodes_end = nodes_end;
        if stable {
            break;
        }
    }

    for slot in std::mem::take(&mut a.walk.conv_fires) {
        trace::compiler_trace_signal(slot);
    }

    let adopt_lines: BTreeMap<String, usize> = {
        let mut m = BTreeMap::new();
        for scope in a.walk.scopes.iter() {
            for (name, shape) in scope {
                let mut best: Option<usize> = None;
                for s in shape.aliases.iter() {
                    if let Some(&l) = a.lattice.adopt_witness.get(s) {
                        best = Some(best.map_or(l, |b: usize| b.min(l)));
                    }
                }
                if let Some(l) = best {
                    m.insert(name.clone(), l);
                }
            }
        }
        m
    };

    a.ledger.diagnostics.clear();
    let (static_elems, elems_diags) = a.compute_static_elems(n);

    let rec: Gate = Some(Recording { _priv: () });
    a.walk.scopes = vec![boundary_root_scope()];
    a.walk.fn_scopes = vec![BTreeMap::new()];
    a.walk.prov = vec![BTreeMap::new()];
    a.walk.scalar_nodes = vec![BTreeMap::new()];
    a.walk.assert_parallel_stacks_aligned();
    a.pin_derived_exprs.clear();
    a.pin_justified_names.clear();

    let mut chk = checker::Checker::new(checker::CheckCtx {
        index_base: a.index_base,
        adopt_lines: adopt_lines.clone(),
        sites: a.lattice.sites.clone(),
        elems: static_elems.clone(),
        fn_def_sites: a.fn_def_sites.clone(),
        fn_captures: a.fn_captures.clone(),
    });
    let mut checker_ran = elems_diags.is_empty();
    let mut last_post: Option<Vec<BTreeMap<String, TableShape>>> = None;
    let mut walk_completed = true;
    for stmt in ctx.ast {
        if let Some(post) = &last_post {
            debug_assert_eq!(
                a.walk.scopes, *post,
                "the interleaving broke: the walk moved between the checker's snapshots"
            );
        }
        chk.snapshot_pre(&a.walk, &a.holds.moved_severed);
        if let Err(err) = a.walk_stmt(rec, stmt) {
            signal!(rec.on(), trace::TRACE_GHOST_BAIL);
            a.ledger
                .diagnostics
                .push(err.0.with_site(Some(stmt.loc().line as usize)));
            checker_ran = false;
            walk_completed = false;
            break;
        }
        chk.snapshot_post(&a.walk, &a.holds.moved_severed);
        last_post = Some(a.walk.scopes.clone());
        a.walk.assert_riders_bound();
        if checker_ran && let Err(msg) = chk.check_stmt(&mut a.lattice.unknowns, stmt) {
            signal!(trace::TRACE_GHOST_BAIL_CHECKER);
            a.ledger
                .diagnostics
                .push(msg.with_site(Some(stmt.loc().line as usize)));
            walk_completed = false;
            break;
        }
    }
    if walk_completed {
        #[cfg(debug_assertions)]
        if let Err(diff) =
            recorded_reproduces_fixpoint(&a.walk.scopes, &prev_end, &a.pin_justified_names)
        {
            panic!(
                "the recorded pass diverged from the fixpoint's converged end state \
                 beyond the chartered pin refinements: {diff}"
            );
        }
    }
    debug_assert!(
        a.lattice.row_ghost_next <= a.lattice.unknowns.next_id() + 1,
        "id-space exhausted: row ghosts crossed the unknown-node minter"
    );
    if checker_ran {
        chk.finish(&mut a.lattice.unknowns);
    }
    let chk_facts = chk.into_facts();

    if let Err(err) = a.decide_root_exit(rec) {
        a.ledger.diagnostics.push(err.0);
    }
    a.plan_stmt_temp_frees(ctx.ast);
    a.mint_fast_lane(ctx.ast);

    let mut facts = a
        .into_recorded(adopt_lines, static_elems, elems_diags, chk_facts)
        .into_facts();
    facts.fn_touched = ctx.fn_touched.clone();
    facts.check_row_reads();
    facts
}

#[cfg(debug_assertions)]
fn recorded_reproduces_fixpoint(
    recorded: &[BTreeMap<String, TableShape>],
    fixpoint: &[BTreeMap<String, TableShape>],
    pin_justified: &BTreeSet<String>,
) -> Result<(), String> {
    if recorded.len() != fixpoint.len() {
        return Err(format!(
            "the scope stack depths diverged: recorded {} vs fixpoint {}",
            recorded.len(),
            fixpoint.len()
        ));
    }
    for (depth, (rec_scope, fix_scope)) in recorded.iter().zip(fixpoint).enumerate() {
        for (name, rec) in rec_scope {
            let Some(fix) = fix_scope.get(name) else {
                return Err(format!(
                    "depth {depth}: '{name}' exists only in the recorded pass"
                ));
            };
            if rec == fix {
                continue;
            }
            let legal = rec.layout == fix.layout
                && rec.aliases == fix.aliases
                && rec.lineage == fix.lineage
                && ty_refines(&rec.ty, &fix.ty);
            if !legal {
                return Err(format!(
                    "depth {depth}: '{name}' is not a monotone refinement — \
                     recorded {rec:?} vs fixpoint {fix:?}"
                ));
            }
            if !pin_justified.contains(name) {
                return Err(format!(
                    "depth {depth}: '{name}' refined to {:?} with no walk pin \
                     accounting for it — recorded {rec:?} vs fixpoint {fix:?}",
                    rec.ty,
                ));
            }
        }
        for name in fix_scope.keys() {
            if !rec_scope.contains_key(name) {
                return Err(format!(
                    "depth {depth}: '{name}' exists only in the fixpoint end state"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(debug_assertions)]
fn ty_refines(rec: &Ty, fix: &Ty) -> bool {
    if rec == fix {
        return true;
    }
    match (rec, fix) {
        (_, Pending) => pinned_scalar(rec),
        (Tbl(rec_elem), Tbl(fix_elem)) => ty_refines(rec_elem, fix_elem),
        _ => false,
    }
}

struct Recorded {
    ledger: Ledger,
    lattice: LatticeState,
    own: OwnershipState,
    reads: ReadSideState,
    layout: LayoutFactsState,
    index_base: i64,
    ctor_lines: Vec<usize>,
    fn_scope_keys: BTreeMap<NodeId, (NodeId, u8)>,
    join_tags: BTreeMap<NodeId, bool>,
    call_tags: BTreeMap<NodeId, Vec<bool>>,
    adopt_lines: BTreeMap<String, usize>,
    static_elems: Vec<crate::shape::Ty>,
    elems_diags: Vec<Diagnostic>,
    chk: checker::CheckFacts,
}

impl Analyzer {
    fn into_recorded(
        self,
        adopt_lines: BTreeMap<String, usize>,
        static_elems: Vec<crate::shape::Ty>,
        elems_diags: Vec<Diagnostic>,
        chk: checker::CheckFacts,
    ) -> Recorded {
        Recorded {
            ledger: self.ledger,
            lattice: self.lattice,
            own: self.own,
            reads: self.reads,
            layout: self.layout,
            index_base: self.index_base,
            ctor_lines: self.ctor_lines,
            fn_scope_keys: self.fn_scope_keys,
            join_tags: self.join_tags,
            call_tags: self.call_tags,
            adopt_lines,
            static_elems,
            elems_diags,
            chk,
        }
    }

    fn compute_static_elems(&mut self, n: usize) -> (Vec<crate::shape::Ty>, Vec<Diagnostic>) {
        let mut elems: Vec<crate::shape::Ty> = (0..n).map(crate::shape::Ty::Unknown).collect();

        let site_ids: Vec<usize> = self.lattice.sites.values().copied().collect();
        for id in site_ids {
            if self.lattice.needed[id] && self.lattice.site_elem[id] == Pending {
                if self.fn_body_sites.contains(&id) || self.fn_param_sites.contains(&id) {
                    signal!(trace::TRACE_ANALYZE_PENDING);
                    elems[id] = crate::shape::Ty::Unknown(id);
                    continue;
                }
                if self.reads.boundary_fed.contains(&id) {
                    signal!(trace::TRACE_ANALYZE_PENDING);
                    elems[id] = crate::shape::Ty::Unknown(id);
                    continue;
                }
                signal!(trace::TRACE_ANALYZE_MISSING_VALUE);
                let line = self.ctor_lines[id];
                self.ledger.diagnostics.push(
                    Diagnostic::ty(format!(
                        "a table is read before it is ever given a value (table site #{id})"
                    ))
                    .with_site(Some(line)),
                );
                elems[id] = crate::shape::Ty::Unknown(id);
            } else if self.lattice.site_elem[id] == Pending {
                signal!(trace::TRACE_ANALYZE_PENDING);
                elems[id] = crate::shape::Ty::Unknown(id);
            } else {
                signal!(trace::TRACE_ANALYZE_RESOLVED);
                elems[id] = self.elem_type_of_tbl(id);
            }
        }

        for (&site, name) in &self.reads.nested_store_bases {
            if !self.reads.tbl_value_stores.contains(&site) {
                signal!(trace::TRACE_STMT_IDX_NULL_ROW);
                let line = self.ctor_lines[site];
                self.ledger.diagnostics.push(
                    Diagnostic::ty(format!(
                        "cannot index a row of '{name}' — no table is ever \
                         stored into '{name}', and stores do not create rows (table site #{site})"
                    ))
                    .with_site(Some(line)),
                );
            }
        }

        for (base, (value, line)) in &self.holds.cyclic_stores {
            signal!(trace::TRACE_FAIL_CYCLIC_STORE);
            self.ledger.diagnostics.push(
                Diagnostic::ty(format!(
                    "cyclic table store — '{value}' stored into '{base}' \
                     would create a table cycle (alm's element types are finite)"
                ))
                .with_site(Some(*line)),
            );
        }

        for id in 0..n {
            propagate_elem_down(&mut elems, &self.lattice.child_sites, id);
        }

        let elems_diags: Vec<Diagnostic> = std::mem::take(&mut self.ledger.diagnostics);
        (elems, elems_diags)
    }
}

impl Recorded {
    fn into_facts(mut self) -> ShapeFacts {
        let mut diagnostics = std::mem::take(&mut self.ledger.diagnostics);
        diagnostics.append(&mut self.elems_diags);
        diagnostics.append(&mut self.chk.diagnostics);

        let substitutions = self.lattice.unknowns.take_subs();

        let mut cell_children = self.lattice.child_sites.clone();
        for (parent, kids) in std::mem::take(&mut self.reads.row_links) {
            cell_children.entry(parent).or_default().extend(kids);
        }

        // The second-writer contract's consumption marking: a reserve
        // plan's pre-header name whose reserve an ffi.new mint over
        // that name already satisfied skips its own reserve — the
        // mint's reserve is the one reserve.
        let mint_caps: BTreeSet<&str> = self
            .chk
            .ffi_mints
            .values()
            .filter(|m| !m.name.is_empty())
            .map(|m| m.name.as_str())
            .collect();
        for plan in self.layout.reserve_plans.values_mut() {
            plan.pre_satisfied = plan
                .pre_header
                .iter()
                .filter(|n| mint_caps.contains(n.as_str()))
                .cloned()
                .collect();
            if !plan.pre_satisfied.is_empty() {
                signal!(trace::TRACE_PLAN_RESERVE_PRE_SATISFIED);
            }
        }

        ShapeFacts {
            index_base: self.index_base,
            sites: self.lattice.sites,
            elems: self.static_elems,
            boundary_elem: self.chk.boundary_elem,
            layouts: self.layout.verdicts,
            free_sites: self.own.free_sites,
            free_keeps: self.own.free_keeps,
            do_exit_frees: self.own.do_exit_frees,
            root_exit_frees: self.own.root_exit_frees,
            stmt_temp_frees: self.own.stmt_temp_frees,
            cond_temp_frees: self.own.cond_temp_frees,
            arm_temp_frees: self.own.arm_temp_frees,
            ret_path_frees: self.own.ret_path_frees,
            break_path_frees: self.own.break_path_frees,
            ret_path_keep_frees: self.own.ret_path_keep_frees,
            ret_path_keep_temps: self.own.ret_path_keep_temps,
            rebind_frees: self.own.rebind_grants,
            move_poisons: self.own.move_poisons,
            fast_stores: self.layout.fast_stores,
            reserve_plans: self.layout.reserve_plans,
            stored_ctor_parents: self.own.stored_ctor_parents,
            substitutions,
            local_types: self.chk.local_types,
            row_reads: std::mem::take(&mut self.reads.row_reads),
            cell_children,
            fn_scope_keys: self.fn_scope_keys,
            join_tags: self.join_tags,
            call_tags: self.call_tags,
            store_ghost_keeps: self.own.store_ghost_keeps,
            ctor_ghost_keeps: self.own.ctor_ghost_keeps,
            arm_free_sites: self.own.arm_free_sites,
            arm_rebind_frees: self.own.arm_rebind_frees,
            arm_do_exit_frees: self.own.arm_do_exit_frees,
            arm_root_exit_frees: self.own.arm_root_exit_frees,
            fn_touched: BTreeMap::new(),
            fn_defs: self.chk.fn_defs,
            env_faces: self.chk.env_faces,
            closure_fields: self.chk.closure_fields,
            call_fns: self.chk.call_fns,
            ffi_verbs: self.chk.ffi_verbs,
            builtin_verbs: self.chk.builtin_verbs,
            ffi_mints: self.chk.ffi_mints,
            instances: self.chk.instances,
            ctor_lines: self.ctor_lines,
            adopt_lines: self.adopt_lines,
            diagnostics,
        }
    }
}

fn get_base_identifier(expr: &Expr) -> Option<&String> {
    // C: index-chain classifier — same law as stmt_base above
    match expr {
        Expr::Identifier { name, .. } => Some(name),
        Expr::Index { obj, .. } => get_base_identifier(obj),
        Expr::Integer { .. }
        | Expr::Float { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Nil { .. }
        | Expr::TableCtor { .. }
        | Expr::BinaryOp { .. }
        | Expr::UnaryOp { .. }
        | Expr::SysAllocCount { .. }
        | Expr::SysExecCount { .. }
        | Expr::FfiModule { .. }
        | Expr::AsyncModule { .. }
        | Expr::Field { .. }
        | Expr::Function { .. }
        | Expr::Call { .. } => None,
    }
}

pub(super) fn value_call_msg(name: &str) -> Diagnostic {
    Diagnostic::ty(format!(
        "'{name}' is not an expansion — a call through a value \
         needs a closure"
    ))
}

pub(super) fn dotted_callee(callee: &Expr) -> Option<String> {
    let Expr::Field { obj, name, .. } = callee else {
        return None;
    };
    let base = match obj.as_ref() {
        Expr::Identifier { name: b, .. } => b.clone(),
        Expr::Field { .. } => dotted_callee(obj.as_ref())?,
        _ => return None,
    };
    Some(format!("{base}.{name}"))
}

pub(super) fn calls_through_fn_var_msg() -> Diagnostic {
    Diagnostic::ty(
        "calls go through a macro's name — only macros \
     assigned directly to variables are callable",
    )
}
