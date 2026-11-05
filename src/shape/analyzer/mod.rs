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
use Ty::{Any, Bool, Conflict, Coro, Ffi, FfiLib, Flt, Int, Pending, Str, Tbl};

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

fn collect_body_sites(stmts: &[Stmt], sites: &BTreeMap<NodeId, usize>, out: &mut BTreeSet<usize>) {
    fn walk_stmt(s: &Stmt, sites: &BTreeMap<NodeId, usize>, out: &mut BTreeSet<usize>) {
        match s {
            Stmt::LocalDecl { exprs, .. } | Stmt::Print { exprs, .. } => {
                exprs.iter().for_each(|e| walk_expr(e, sites, out))
            }
            Stmt::Assignment { expr, .. } | Stmt::Expr { expr, .. } => walk_expr(expr, sites, out),
            Stmt::Break { .. } => {}
            Stmt::IndexAssign {
                obj, key, value, ..
            } => {
                walk_expr(obj, sites, out);
                walk_expr(key, sites, out);
                walk_expr(value, sites, out);
            }
            Stmt::While {
                condition, body, ..
            } => {
                walk_expr(condition, sites, out);
                body.iter().for_each(|s| walk_stmt(s, sites, out));
            }
            Stmt::Do { body, .. } => body.iter().for_each(|s| walk_stmt(s, sites, out)),
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                walk_expr(condition, sites, out);
                then_body.iter().for_each(|s| walk_stmt(s, sites, out));
                else_body.iter().for_each(|s| walk_stmt(s, sites, out));
            }
            Stmt::Return { value, .. } => {
                if let Some(e) = value {
                    walk_expr(e, sites, out);
                }
            }
        }
    }
    fn walk_expr(e: &Expr, sites: &BTreeMap<NodeId, usize>, out: &mut BTreeSet<usize>) {
        if let Some(&id) = sites.get(&e.id()) {
            out.insert(id);
        }
        match e {
            Expr::TableCtor { entries, .. } => {
                for (k, v) in entries {
                    if let CtorKey::Expr(ke) = k {
                        walk_expr(ke, sites, out);
                    }
                    walk_expr(v, sites, out);
                }
            }
            Expr::Index { obj, key, .. } => {
                walk_expr(obj, sites, out);
                walk_expr(key, sites, out);
            }
            Expr::BinaryOp { left, right, .. } => {
                walk_expr(left, sites, out);
                walk_expr(right, sites, out);
            }
            Expr::UnaryOp { expr, .. } => walk_expr(expr, sites, out),
            Expr::Call { callee, args, .. } => {
                walk_expr(callee, sites, out);
                args.iter().for_each(|a| walk_expr(a, sites, out));
            }
            Expr::Function { body, .. } => body.iter().for_each(|s| walk_stmt(s, sites, out)),
            _ => {}
        }
    }
    stmts.iter().for_each(|s| walk_stmt(s, sites, out));
}

fn collect_stored_bases(stmts: &[Stmt]) -> BTreeSet<String> {
    fn walk_stmt(s: &Stmt, out: &mut BTreeSet<String>) {
        match s {
            Stmt::IndexAssign { obj, .. } => {
                if let Some(base) = stmt_base(obj) {
                    out.insert(base);
                }
                walk_obj_value(s, out);
            }
            Stmt::LocalDecl { exprs, .. } | Stmt::Print { exprs, .. } => {
                exprs.iter().for_each(|e| walk_expr(e, out))
            }
            Stmt::Assignment { expr, .. } | Stmt::Expr { expr, .. } => walk_expr(expr, out),
            Stmt::While {
                condition, body, ..
            } => {
                walk_expr(condition, out);
                body.iter().for_each(|s| walk_stmt(s, out));
            }
            Stmt::Do { body, .. } => body.iter().for_each(|s| walk_stmt(s, out)),
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                walk_expr(condition, out);
                then_body.iter().for_each(|s| walk_stmt(s, out));
                else_body.iter().for_each(|s| walk_stmt(s, out));
            }
            Stmt::Return { value, .. } => {
                if let Some(e) = value {
                    walk_expr(e, out);
                }
            }
            Stmt::Break { .. } => {}
        }
    }
    fn walk_obj_value(s: &Stmt, out: &mut BTreeSet<String>) {
        if let Stmt::IndexAssign {
            obj, key, value, ..
        } = s
        {
            walk_expr(obj, out);
            walk_expr(key, out);
            walk_expr(value, out);
        }
    }
    fn walk_expr(e: &Expr, out: &mut BTreeSet<String>) {
        match e {
            Expr::TableCtor { entries, .. } => {
                for (k, v) in entries {
                    if let CtorKey::Expr(ke) = k {
                        walk_expr(ke, out);
                    }
                    walk_expr(v, out);
                }
            }
            Expr::Index { obj, key, .. } => {
                walk_expr(obj, out);
                walk_expr(key, out);
            }
            Expr::BinaryOp { left, right, .. } => {
                walk_expr(left, out);
                walk_expr(right, out);
            }
            Expr::UnaryOp { expr, .. } => walk_expr(expr, out),
            Expr::Call { callee, args, .. } => {
                walk_expr(callee, out);
                args.iter().for_each(|a| walk_expr(a, out));
            }
            Expr::Function { body, .. } => body.iter().for_each(|s| walk_stmt(s, out)),
            _ => {}
        }
    }
    fn stmt_base(e: &Expr) -> Option<String> {
        match e {
            Expr::Identifier { name: n, .. } => Some(n.clone()),
            Expr::Index { obj, .. } => stmt_base(obj),
            _ => None,
        }
    }
    let mut out = BTreeSet::new();
    stmts.iter().for_each(|s| walk_stmt(s, &mut out));
    out
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

pub fn analyze_with(ctx: &AnalysisContext<'_>, walk_pins: bool) -> ShapeFacts {
    let n = ctx.sites.len();
    let mut a = Analyzer {
        walk_pins,
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
    let mut dbg_pass: u32 = 0;
    loop {
        dbg_pass += 1;
        crate::alm_dbgln!("[fixpoint] pass {dbg_pass}");
        a.walk.changed = false;
        a.walk.scopes = vec![boundary_root_scope()];
        a.walk.fn_scopes = vec![BTreeMap::new()];
        a.walk.prov = vec![BTreeMap::new()];
        a.walk.scalar_nodes = vec![BTreeMap::new()];
        a.fn_def_sites.clear();
        a.fn_captures.clear();
        a.walk_stmts(None, ctx.ast);
        let end = a.walk.scopes.clone();
        let stable = end == prev_end && !a.walk.changed;
        prev_end = end;
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
    a.pin_derived_exprs.clear();
    a.pin_justified_names.clear();

    let mut chk = checker::Checker::new(checker::CheckCtx {
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

        ShapeFacts {
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
            instances: self.chk.instances,
            ctor_lines: self.ctor_lines,
            adopt_lines: self.adopt_lines,
            diagnostics,
        }
    }
}

fn get_base_identifier(expr: &Expr) -> Option<&String> {
    match expr {
        Expr::Identifier { name, .. } => Some(name),
        Expr::Index { obj, .. } => get_base_identifier(obj),
        _ => None,
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
