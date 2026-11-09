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
    /// The plan's minted lower bound on the same entry (an
    /// under-approximation through Var chains): the gate reads the
    /// exact value when known, this bound when not.
    entry_lb: Option<i64>,
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

/// One plan's entry facts for the lane: the pre-header and nested
/// name sets plus the minted lower bound on the loop entry.
type PlanSets = (BTreeSet<String>, BTreeSet<String>, Option<i64>);

struct LaneSink<'a> {
    plans: &'a BTreeMap<NodeId, PlanSets>,
    fast: BTreeMap<NodeId, FastStoreElig>,
    ctor: BTreeMap<NodeId, BTreeSet<NodeId>>,
    /// The surface index base the walk runs under; `GuardOffset` shapes
    /// arrive pre-folded from recognition, and a bare `Guard`'s physical
    /// offset `-index_base` is derived at its consumers here.
    index_base: i64,
    /// The claim groups: one per (loop, base). A group's first sighting
    /// of a KeyShape rides the lane; a later sighting of the same shape
    /// is a same-slot repeat (the P21 law) and declines, while a
    /// distinct shape collapses the group. With more than one key shape
    /// on the same (loop, base), the store windows overlap across trips:
    /// at trip i the guard store writes slot i and the offset store
    /// writes slot i + k, and the next trip's guard store clobbers the
    /// offset row of trip i - 1 with no runtime face in between — not
    /// even the first sighting is guaranteed a fresh slot then, so the
    /// whole group falls to the checked face (its replace-reclaim speaks
    /// the displacement law) and widens nothing.
    claims: BTreeMap<(NodeId, String), LaneClaims>,
    /// Every base that took a row kind store earlier in the walk. The
    /// table the name binds holds rows at those slots: a later loop's
    /// first sighting on the same base is not proven a fresh slot, so it
    /// keeps the checked face too (the sequential-fill twin of the group
    /// law). Scalar-only fills never enter the set — the cells-hold-one-
    /// kind law forbids a row-and-scalar mix on one table binding —
    /// so plain integer conversion loops stay claimed across fills.
    row_stored: BTreeSet<String>,
    /// Runtime loop nesting at the scan: incremented entering every
    /// while body (planned or not), decremented leaving it. A claim
    /// whose own loop is the only enclosing loop owns the whole trip
    /// dimension over its body; deeper claims re-run their store
    /// domain once per enclosing trip, and the slots the claim touches
    /// are the enclosing trips' rows — nothing to prove fresh there.
    loop_depth: u32,
}

/// One (loop, base) group. `shapes` carries every resolvable key shape
/// the walk saw; `minted` pairs the facts' statement ids with their
/// physical offsets (the plan's reserve widening reads only survivors);
/// `poisoned` is the group-wide decline once a distinct shape or an
/// unanalyzable store reached it.
#[derive(Default)]
struct LaneClaims {
    shapes: BTreeSet<KeyShape>,
    minted: Vec<(NodeId, i64)>,
    poisoned: bool,
}

impl Analyzer {
    /// Mint the fast-store facts over the whole program now that every
    /// conversion loop's reserve plan is final: store eligibility
    /// depends on inner loops' plans too, so it cannot ride the walk
    /// itself.
    pub(super) fn mint_fast_lane(&mut self, stmts: &[Stmt]) {
        let sets: BTreeMap<NodeId, PlanSets> = self
            .layout
            .reserve_plans
            .iter()
            .map(|(&id, p)| {
                (
                    id,
                    (
                        p.pre_header.iter().cloned().collect(),
                        p.nested.iter().cloned().collect(),
                        p.entry_lb,
                    ),
                )
            })
            .collect();
        let mut sink = LaneSink {
            plans: &sets,
            fast: BTreeMap::new(),
            ctor: BTreeMap::new(),
            claims: BTreeMap::new(),
            row_stored: BTreeSet::new(),
            loop_depth: 0,
            index_base: self.index_base,
        };
        scan_stmts(stmts, &mut Vec::new(), &mut ConstEnv::new(), &mut sink);
        for (loop_id, hits) in sink.ctor {
            if let Some(plan) = self.layout.reserve_plans.get_mut(&loop_id) {
                plan.ctor_reserves = hits;
            }
        }
        // The plan's widest offset comes from the surviving mints only:
        // a collapsed group's facts were unclaimed (its stores ride the
        // checked face), and a never-minted sighting contributes
        // nothing, so the reserves keep the raw bound when nothing was
        // proven.
        let mut key_offsets: BTreeMap<NodeId, Option<i64>> = BTreeMap::new();
        for ((loop_id, _), rec) in &sink.claims {
            if rec.poisoned {
                continue;
            }
            for (_, k) in &rec.minted {
                let entry = key_offsets.entry(*loop_id).or_insert(None);
                if entry.is_none_or(|current| *k > current) {
                    *entry = Some(*k);
                }
            }
        }
        for (loop_id, max_k) in key_offsets {
            if let Some(plan) = self.layout.reserve_plans.get_mut(&loop_id) {
                plan.key_offset = max_k;
            }
        }
        self.layout.fast_stores = sink.fast;
    }
}

/// One resolvable sighting's group verdict: `FallThrough` continues
/// the mint flow, `Decline` stops the frame walk, and `Collapse` has
/// already unclaimed the group's minted facts in place.
enum Sighting {
    FallThrough,
    Decline,
    Collapse,
}

impl LaneSink<'_> {
    /// The group decline for one (loop, base): every minted fact
    /// unclaims (its store compiles to the checked face) and the group
    /// declines every later sighting — same-shape repeats included,
    /// whose checked stores keep set_core's replace-reclaim. A
    /// poisoned group stays dead even when the poison arrived at a
    /// sighting that minted nothing: a later same-shape sighting gets
    /// no claim from it either.
    fn poison(&mut self, loop_id: NodeId, base: &str) {
        let rec = self.claims.entry((loop_id, base.to_string())).or_default();
        if rec.poisoned {
            return;
        }
        let dead = std::mem::take(&mut rec.minted);
        rec.poisoned = true;
        for (stmt_id, _) in dead {
            self.fast.remove(&stmt_id);
        }
    }

    /// Book one sighting into its (loop, base) group. The poisoned
    /// group declines. A same-shape repeat is the P21 law: the first
    /// sighting of a slot keeps the claim (its slot is fresh), the
    /// repeats ride the checked face. A distinct new shape after a
    /// mint collapses the group, because the two store windows overlap
    /// across trips — at trip i the guard store writes slot i and the
    /// offset store writes slot i + k, and the next trip's guard store
    /// clobbers the offset row with no runtime face in between — so
    /// nothing already minted can stay.
    fn book_sighting(&mut self, loop_id: NodeId, base: &str, shape: KeyShape) -> Sighting {
        let rec = self.claims.entry((loop_id, base.to_string())).or_default();
        if rec.poisoned {
            return Sighting::Decline;
        }
        if !rec.shapes.insert(shape) {
            return Sighting::Decline;
        }
        if !rec.minted.is_empty() {
            let dead = std::mem::take(&mut rec.minted);
            rec.poisoned = true;
            for (stmt_id, _) in dead {
                self.fast.remove(&stmt_id);
            }
            return Sighting::Collapse;
        }
        Sighting::FallThrough
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
            let row_kind = matches!(value, Expr::TableCtor { .. });
            claim_store(stmt.id(), obj, key, row_kind, frames, sink);
            if row_kind && let Some(base) = get_base_identifier(obj) {
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
                && let Some((pre_header, nested, entry_lb)) = sink.plans.get(&stmt.id())
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
                    entry_lb: *entry_lb,
                });
                pushed = true;
            }
            sink.loop_depth += 1;
            scan_block(body, frames, env, sink);
            sink.loop_depth -= 1;
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

/// One store site's lane laws. Pass one is the poison service (an
/// unanalyzable writer collapses the groups it reaches); pass two
/// books the sighting and, when every gate admits, mints the fast
/// fact. A row-kind value marks the base afterwards.
fn claim_store(
    stmt_id: NodeId,
    obj: &Expr,
    key: &Expr,
    row_kind: bool,
    frames: &[LaneFrame],
    sink: &mut LaneSink,
) {
    let Some(base) = get_base_identifier(obj) else {
        return;
    };
    // Pass one, the poison service: a plain-host store whose key
    // resolves under no live frame ("t[expr] = v") is a runtime writer
    // of unprovable slots, so every (loop, base) group it reaches
    // collapses — including a group minted earlier in this walk, whose
    // fresh-slot proof the unanalyzable writer undoes — and stays
    // dead for sightings that come later in the walk. A compound host
    // (`t[i][k] = v`) writes inside a row's cells, never `t`'s own
    // slots, so it poisons nothing.
    if matches!(obj, Expr::Identifier { .. }) {
        for frame in frames.iter().rev() {
            if guard_key_shape(key, &frame.guard, sink.index_base).is_none() {
                sink.poison(frame.loop_id, base);
            }
        }
    }
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
        match sink.book_sighting(frame.loop_id, base, shape) {
            Sighting::Decline | Sighting::Collapse => continue,
            Sighting::FallThrough => {}
        }
        // One admission law for every key shape: the entry's exact
        // constant when known, the plan's minted lower bound when not,
        // and `first + k_phys >= 0` on the checked sum. A minted plan
        // passed `guard_provably_nonneg`, and a provably nonnegative
        // entry under-approximates to a bound `>= 0` — so a base-0
        // bare guard (offset 0) admits through the same formula
        // without an unconditional arm. An entry with neither a
        // constant nor a bound declines: the fast path must never
        // turn a negative key into a negative getelementptr.
        let first = frame.entry_const.or(frame.entry_lb);
        let admitted = matches!(first, Some(c) if c.checked_add(k_phys).is_some_and(|v| v >= 0));
        if !admitted {
            continue;
        }
        // The row-stored law (the sequential twin of the group law): a
        // row store into a base whose rows the walk already populated
        // is not proven a fresh slot, so it keeps the checked face,
        // whose replace-reclaim frees the displaced child. Scalar
        // values stay eligible: replacing a scalar needs no reclaim,
        // and the cells-hold-one-kind law forbids a row-and-scalar mix
        // on one table binding — so plain integer fills keep riding
        // the lane across fills.
        if row_kind && sink.row_stored.contains(base) {
            continue;
        }
        // The depth law: a claim whose own loop is the only enclosing
        // loop owns the whole trip dimension over it; a deeper claim
        // re-runs its store domain once per enclosing trip, and the
        // slots it touches are the enclosing trips' rows. Deeper
        // claims keep the checked face.
        if sink.loop_depth != 1 {
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
            if let Some(rec) = sink.claims.get_mut(&(frame.loop_id, base.clone())) {
                rec.minted.push((stmt_id, k_phys));
            }
            if row_kind {
                sink.row_stored.insert(base.clone());
            }
            return;
        }
        if frame.nested.contains(base) && matches!(obj, Expr::Identifier { .. }) {
            // A nested claim's host liveness is only a binding fact: a
            // plain-name host names the binding the plan reserved, so
            // the fast store's host register is the live table object
            // every trip. A compound host reads a runtime slot at store
            // time (`alm_tbl_get` of `t[k]`), and the slot's liveness is
            // a claim the nested plan never minted: a refusable writer
            // elsewhere in the body leaves that slot's zero word as the
            // read's answer, and the fast lane's header deref would
            // SIGSEGV on the null host with no runtime face in between.
            // Those stores ride the checked face, whose null arm speaks
            // the null-row-store guard — the faithful "attempt to index
            // a nil value" — instead of a bare fault.
            sink.fast.insert(
                stmt_id,
                FastStoreElig {
                    loop_id: frame.loop_id,
                    table: base.clone(),
                    key: shape,
                    row_reserve: true,
                },
            );
            if let Some(rec) = sink.claims.get_mut(&(frame.loop_id, base.clone())) {
                rec.minted.push((stmt_id, k_phys));
            }
            if row_kind {
                sink.row_stored.insert(base.clone());
            }
            return;
        }
    }
    // The mint-less path (a declined sighting, a collapsed group, an
    // unclaimable host, a shapeless base) still records a row store:
    // the base the name binds holds rows from here on, fail-closed,
    // name reuse included. A compound host writes inside a row's
    // cells and marks nothing.
    if row_kind && matches!(obj, Expr::Identifier { .. }) {
        sink.row_stored.insert(base.clone());
    }
}

/// Function bodies ride their own emission contexts, never the lanes
/// or the constants around their literals (a conversion loop's body
/// admits no calls), so both stacks start empty inside them. The
/// function's own stores face the function's trip counters only: the
/// depth counter resets for the body's scan and restores after.
/// `row_stored` stays walk-wide — name reuse across a call boundary
/// keeps the fail-closed decline.
fn scan_expr(expr: &Expr, sink: &mut LaneSink) {
    match expr {
        Expr::Function { body, .. } => {
            let saved_depth = sink.loop_depth;
            sink.loop_depth = 0;
            let mut fresh = Vec::new();
            scan_stmts(body, &mut fresh, &mut ConstEnv::new(), sink);
            sink.loop_depth = saved_depth;
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
