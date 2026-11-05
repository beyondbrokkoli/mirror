use super::*;

/// One enclosing conversion loop while the post-walk scan runs. The
/// frame mirrors the emission-time context the lowerer used to keep:
/// the plan's name sets, plus the guard-stability and shadowing facts
/// measured from THIS loop's header, which is where the old
/// register-identity match took its guard register.
#[derive(Clone)]
struct LaneFrame {
    loop_id: NodeId,
    guard: String,
    pre_header: BTreeSet<String>,
    nested: BTreeSet<String>,
    /// The guard name was assigned somewhere since this loop's header;
    /// assignments persist past every enclosing block's end.
    guard_assigned: bool,
    /// The guard name was shadowed by a declaration inside the current
    /// block; shadows lift when the block ends.
    guard_shadow: bool,
    /// Names whose current binding differs from the binding this loop's
    /// header saw; block-scoped, like `guard_shadow`.
    shadowed: BTreeSet<String>,
}

impl LaneFrame {
    fn guard_live(&self) -> bool {
        !self.guard_assigned && !self.guard_shadow
    }
}

/// The per-frame state a block entry snapshots and its exit restores:
/// shadow effects lift at the block's end, assignment kills never do.
type LaneSnapshot = Vec<(bool, bool, BTreeSet<String>)>;

fn snapshot(frames: &[LaneFrame]) -> LaneSnapshot {
    frames
        .iter()
        .map(|f| (f.guard_assigned, f.guard_shadow, f.shadowed.clone()))
        .collect()
}

fn restore(frames: &mut [LaneFrame], saved: &LaneSnapshot) {
    for (f, s) in frames.iter_mut().zip(saved) {
        f.guard_shadow = s.1;
        f.shadowed = s.2.clone();
    }
}

struct LaneSink<'a> {
    plans: &'a BTreeMap<NodeId, (BTreeSet<String>, BTreeSet<String>)>,
    fast: BTreeMap<NodeId, FastStoreElig>,
    ctor: BTreeMap<NodeId, BTreeSet<NodeId>>,
}

impl Analyzer {
    /// Mint the fast-store facts over the whole program now that every
    /// conversion loop's reserve plan is final: store eligibility
    /// depends on inner loops' plans too, so it cannot ride the walk.
    pub(super) fn mint_fast_lane(&mut self, stmts: &[Stmt]) {
        let sets: BTreeMap<NodeId, (BTreeSet<String>, BTreeSet<String>)> = self
            .layout
            .reserve_plans
            .iter()
            .map(|(&id, p)| {
                (
                    id,
                    (
                        p.pre_header.iter().cloned().collect(),
                        p.nested.iter().cloned().collect(),
                    ),
                )
            })
            .collect();
        let mut sink = LaneSink {
            plans: &sets,
            fast: BTreeMap::new(),
            ctor: BTreeMap::new(),
        };
        scan_stmts(stmts, &mut Vec::new(), &mut sink);
        for (loop_id, hits) in sink.ctor {
            if let Some(plan) = self.layout.reserve_plans.get_mut(&loop_id) {
                plan.ctor_reserves = hits;
            }
        }
        self.layout.fast_stores = sink.fast;
    }
}

fn scan_stmts(stmts: &[Stmt], frames: &mut Vec<LaneFrame>, sink: &mut LaneSink) {
    for stmt in stmts {
        scan_stmt(stmt, frames, sink);
    }
}

fn scan_stmt(stmt: &Stmt, frames: &mut Vec<LaneFrame>, sink: &mut LaneSink) {
    match stmt {
        Stmt::LocalDecl { names, exprs, .. } => {
            for expr in exprs {
                scan_expr(expr, sink);
            }
            for name in names {
                for frame in frames.iter_mut() {
                    if frame.guard == *name {
                        frame.guard_shadow = true;
                    }
                    frame.shadowed.insert(name.clone());
                }
            }
        }
        Stmt::Assignment { name, expr, .. } => {
            scan_expr(expr, sink);
            for frame in frames.iter_mut() {
                if frame.guard == *name {
                    frame.guard_assigned = true;
                }
            }
        }
        Stmt::IndexAssign {
            obj, key, value, ..
        } => {
            scan_expr(obj, sink);
            scan_expr(key, sink);
            scan_expr(value, sink);
            claim_store(stmt.id(), obj, key, frames, sink);
            if matches!(value, Expr::TableCtor { .. })
                && let Some(base) = get_base_identifier(obj)
            {
                for frame in frames.iter().rev() {
                    if frame.nested.contains(base) {
                        sink.ctor
                            .entry(frame.loop_id)
                            .or_default()
                            .insert(stmt.id());
                        break;
                    }
                }
            }
        }
        Stmt::While {
            condition, body, ..
        } => {
            scan_expr(condition, sink);
            let mut pushed = false;
            if let Some((guard, _)) = extract_guard(condition)
                && let Some((pre_header, nested)) = sink.plans.get(&stmt.id())
            {
                frames.push(LaneFrame {
                    loop_id: stmt.id(),
                    guard: guard.to_string(),
                    pre_header: pre_header.clone(),
                    nested: nested.clone(),
                    guard_assigned: false,
                    guard_shadow: false,
                    shadowed: BTreeSet::new(),
                });
                pushed = true;
            }
            scan_block(body, frames, sink);
            if pushed {
                frames.pop();
            }
        }
        Stmt::Do { body, .. } => {
            scan_block(body, frames, sink);
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            scan_expr(condition, sink);
            let saved = snapshot(frames);
            let mut then_frames = frames.clone();
            scan_stmts(then_body, &mut then_frames, sink);
            let mut else_frames = frames.clone();
            scan_stmts(else_body, &mut else_frames, sink);
            for (i, frame) in frames.iter_mut().enumerate() {
                frame.guard_assigned =
                    saved[i].0 || then_frames[i].guard_assigned || else_frames[i].guard_assigned;
            }
            restore(frames, &saved);
        }
        Stmt::Print { exprs, .. } => {
            for e in exprs {
                scan_expr(e, sink);
            }
        }
        Stmt::Expr { expr, .. } => scan_expr(expr, sink),
        Stmt::Return { value, .. } => {
            if let Some(e) = value {
                scan_expr(e, sink);
            }
        }
        Stmt::Break { .. } => {}
    }
}

/// A nested block's statements run in sequence, and whatever they
/// assigned leaks out past the block's end (the lowerer updates the
/// visible binding in place), while shadows lift at the end.
fn scan_block(stmts: &[Stmt], frames: &mut Vec<LaneFrame>, sink: &mut LaneSink) {
    let saved = snapshot(frames);
    scan_stmts(stmts, frames, sink);
    for (frame, s) in frames.iter_mut().zip(&saved) {
        frame.guard_assigned = frame.guard_assigned || s.0;
    }
    restore(frames, &saved);
}

fn claim_store(stmt_id: NodeId, obj: &Expr, key: &Expr, frames: &[LaneFrame], sink: &mut LaneSink) {
    let Some(base) = get_base_identifier(obj) else {
        return;
    };
    for frame in frames.iter().rev() {
        if !frame.guard_live() {
            continue;
        }
        if !matches!(
            key,
            Expr::Identifier { name, .. } if name == &frame.guard
        ) {
            continue;
        }
        // A plain-name store on a pre-header-reserved table rides the
        // existing reserve when the name still names the binding the
        // loop's header saw; anything the plan holds as a nested row
        // reserves its own row at the store.
        if matches!(obj, Expr::Identifier { .. })
            && frame.pre_header.contains(base)
            && !frame.shadowed.contains(base)
        {
            sink.fast.insert(
                stmt_id,
                FastStoreElig {
                    loop_id: frame.loop_id,
                    table: base.clone(),
                    key: KeyShape::Guard,
                    row_reserve: false,
                },
            );
            return;
        }
        if frame.nested.contains(base) {
            sink.fast.insert(
                stmt_id,
                FastStoreElig {
                    loop_id: frame.loop_id,
                    table: base.clone(),
                    key: KeyShape::Guard,
                    row_reserve: true,
                },
            );
            return;
        }
    }
}

/// Function bodies ride their own emission contexts, never the lanes
/// around their literals (a conversion loop's body admits no calls), so
/// the frame stack starts empty inside them.
fn scan_expr(expr: &Expr, sink: &mut LaneSink) {
    match expr {
        Expr::Function { body, .. } => {
            let mut fresh = Vec::new();
            scan_stmts(body, &mut fresh, sink);
        }
        Expr::TableCtor { entries, .. } => {
            for (k, v) in entries {
                if let CtorKey::Expr(ke) = k {
                    scan_expr(ke, sink);
                }
                scan_expr(v, sink);
            }
        }
        Expr::Index { obj, key, .. } => {
            scan_expr(obj, sink);
            scan_expr(key, sink);
        }
        Expr::BinaryOp { left, right, .. } => {
            scan_expr(left, sink);
            scan_expr(right, sink);
        }
        Expr::UnaryOp { expr, .. } => scan_expr(expr, sink),
        Expr::Field { obj, .. } => scan_expr(obj, sink),
        Expr::Call { callee, args, .. } => {
            scan_expr(callee, sink);
            for a in args {
                scan_expr(a, sink);
            }
        }
        _ => {}
    }
}
