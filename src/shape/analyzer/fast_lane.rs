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
    /// The guard's compile-time entry value at this loop's header, from
    /// the constant environment; offset keys need it to prove their
    /// first key lands at or after zero.
    entry_const: Option<i64>,
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

/// Compile-time integer knowledge of names, mirroring the binding
/// semantics the lowerer's registers carry: a declaration writes the
/// current block's overlay (and lifts at the block's end), an
/// assignment kills the persistent entry outright.
#[derive(Clone)]
struct ConstEnv {
    base: BTreeMap<String, i64>,
    overlay: BTreeMap<String, Option<i64>>,
}

impl ConstEnv {
    fn new() -> Self {
        ConstEnv {
            base: BTreeMap::new(),
            overlay: BTreeMap::new(),
        }
    }

    fn get(&self, name: &str) -> Option<i64> {
        match self.overlay.get(name) {
            Some(Some(v)) => Some(*v),
            Some(None) => None,
            None => self.base.get(name).copied(),
        }
    }

    fn declare(&mut self, name: &str, init: Option<i64>) {
        self.overlay.insert(name.to_string(), init);
    }

    fn assign(&mut self, name: &str) {
        self.base.remove(name);
        if self.overlay.contains_key(name) {
            self.overlay.insert(name.to_string(), None);
        }
    }
}

struct LaneSink<'a> {
    plans: &'a BTreeMap<NodeId, (BTreeSet<String>, BTreeSet<String>)>,
    fast: BTreeMap<NodeId, FastStoreElig>,
    ctor: BTreeMap<NodeId, BTreeSet<NodeId>>,
    /// The widest physical offset admitted per loop, sizing its
    /// reserves. Signed and unclamped: the tight form reserves
    /// `bound + K_eff` cells so the border equals the logical length
    /// under every base.
    offsets: BTreeMap<NodeId, i64>,
    /// The surface index base the walk runs under; `GuardOffset` shapes
    /// arrive pre-folded from recognition, and a bare `Guard`'s physical
    /// offset `-index_base` is derived at its consumers here.
    index_base: i64,
}

impl Analyzer {
    /// Mint the fast-store facts over the whole program now that every
    /// conversion loop's reserve plan is final: store eligibility
    /// depends on inner loops' plans too, so it cannot ride the walk
    /// itself.
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
            offsets: BTreeMap::new(),
            index_base: self.index_base,
        };
        scan_stmts(stmts, &mut Vec::new(), &mut ConstEnv::new(), &mut sink);
        for (loop_id, hits) in sink.ctor {
            if let Some(plan) = self.layout.reserve_plans.get_mut(&loop_id) {
                plan.ctor_reserves = hits;
            }
        }
        for (loop_id, max_k) in sink.offsets {
            if let Some(plan) = self.layout.reserve_plans.get_mut(&loop_id) {
                plan.key_offset = max_k;
            }
        }
        self.layout.fast_stores = sink.fast;
    }
}

fn scan_stmts(
    stmts: &[Stmt],
    frames: &mut Vec<LaneFrame>,
    env: &mut ConstEnv,
    sink: &mut LaneSink,
) {
    for stmt in stmts {
        scan_stmt(stmt, frames, env, sink);
    }
}

fn scan_stmt(stmt: &Stmt, frames: &mut Vec<LaneFrame>, env: &mut ConstEnv, sink: &mut LaneSink) {
    match stmt {
        Stmt::LocalDecl { names, exprs, .. } => {
            for expr in exprs {
                scan_expr(expr, sink);
            }
            for (idx, name) in names.iter().enumerate() {
                let init = match exprs.get(idx) {
                    Some(Expr::Integer { value, .. }) => Some(*value),
                    _ => None,
                };
                env.declare(name, init);
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
            env.assign(name);
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
                    entry_const: env.get(guard),
                });
                pushed = true;
            }
            scan_block(body, frames, env, sink);
            if pushed {
                frames.pop();
            }
        }
        Stmt::Do { body, .. } => {
            scan_block(body, frames, env, sink);
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            scan_expr(condition, sink);
            let saved = snapshot(frames);
            let saved_env = env.clone();
            let mut then_frames = frames.clone();
            let mut then_env = env.clone();
            scan_stmts(then_body, &mut then_frames, &mut then_env, sink);
            let mut else_frames = frames.clone();
            let mut else_env = env.clone();
            scan_stmts(else_body, &mut else_frames, &mut else_env, sink);
            for (i, frame) in frames.iter_mut().enumerate() {
                frame.guard_assigned =
                    saved[i].0 || then_frames[i].guard_assigned || else_frames[i].guard_assigned;
            }
            restore(frames, &saved);
            // A name whose constant either arm lost is not constant at
            // the join; declarations lift with the arms' overlays.
            let lost: Vec<String> = saved_env
                .base
                .keys()
                .filter(|n| {
                    then_env.base.get(*n) != saved_env.base.get(*n)
                        || else_env.base.get(*n) != saved_env.base.get(*n)
                })
                .cloned()
                .collect();
            *env = saved_env;
            for name in lost {
                env.assign(&name);
            }
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
fn scan_block(
    stmts: &[Stmt],
    frames: &mut Vec<LaneFrame>,
    env: &mut ConstEnv,
    sink: &mut LaneSink,
) {
    let saved = snapshot(frames);
    let saved_overlay = env.overlay.clone();
    scan_stmts(stmts, frames, env, sink);
    for (frame, s) in frames.iter_mut().zip(&saved) {
        frame.guard_assigned = frame.guard_assigned || s.0;
    }
    restore(frames, &saved);
    env.overlay = saved_overlay;
}

fn claim_store(stmt_id: NodeId, obj: &Expr, key: &Expr, frames: &[LaneFrame], sink: &mut LaneSink) {
    let Some(base) = get_base_identifier(obj) else {
        return;
    };
    for frame in frames.iter().rev() {
        if !frame.guard_live() {
            continue;
        }
        let Some(shape) = guard_key_shape(key, &frame.guard, sink.index_base) else {
            continue;
        };
        // The key's physical offset: `GuardOffset` arrived pre-folded at
        // recognition; a bare `Guard` sits at `-index_base`.
        let k_phys = match shape {
            KeyShape::Guard => -sink.index_base,
            KeyShape::GuardOffset(k) => k,
        };
        // An offset key must land at or after zero on its first trip:
        // a negative key on the fast path is a negative getelementptr,
        // and the runtime's negative-index no-op law lives only in the
        // checked arm. A bare guard at base 0 is the one unconditional
        // historical case (its offset is zero in physical space); under
        // base 1 the bare guard is an offset key like any other.
        let admitted = match (&shape, k_phys) {
            (KeyShape::Guard, 0) => true,
            _ => {
                matches!(frame.entry_const, Some(c) if c.checked_add(k_phys).is_some_and(|v| v >= 0))
            }
        };
        if !admitted {
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
                    key: shape,
                    row_reserve: false,
                },
            );
            widen(sink, frame.loop_id, k_phys);
            return;
        }
        if frame.nested.contains(base) {
            sink.fast.insert(
                stmt_id,
                FastStoreElig {
                    loop_id: frame.loop_id,
                    table: base.clone(),
                    key: shape,
                    row_reserve: true,
                },
            );
            widen(sink, frame.loop_id, k_phys);
            return;
        }
    }
}

/// The loop's reserves must cover `bound + K_eff` cells for every
/// admitted key: the widest physical offset wins, negative offsets
/// included (the tight form — the border must equal the logical
/// length under every base, and a negative offset's highest write is
/// `bound - 1 + K_eff`).
fn widen(sink: &mut LaneSink, loop_id: NodeId, k_phys: i64) {
    // Seeded at the floor, not zero: a zero seed would silently
    // re-clamp every negative offset back to a `bound`-sized reserve.
    let entry = sink.offsets.entry(loop_id).or_insert(i64::MIN);
    if k_phys > *entry {
        *entry = k_phys;
    }
}

/// Function bodies ride their own emission contexts, never the lanes
/// or the constants around their literals (a conversion loop's body
/// admits no calls), so both stacks start empty inside them.
fn scan_expr(expr: &Expr, sink: &mut LaneSink) {
    match expr {
        Expr::Function { body, .. } => {
            let mut fresh = Vec::new();
            scan_stmts(body, &mut fresh, &mut ConstEnv::new(), sink);
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
