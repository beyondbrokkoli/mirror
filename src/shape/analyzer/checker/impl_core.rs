impl Checker {
    pub(super) fn new(ctx: CheckCtx) -> Self {
        let index_base = ctx.index_base;
        Self {
            frames: Vec::new(),
            root_fns: BTreeMap::new(),
            top: BTreeMap::new(),
            cur_decls: BTreeSet::new(),
            snap: WalkSnap {
                binds: BTreeMap::new(),
                scalars: BTreeMap::new(),
                fns: BTreeMap::new(),
                severed: BTreeMap::new(),
            },
            decl_snap: WalkSnap {
                binds: BTreeMap::new(),
                scalars: BTreeMap::new(),
                fns: BTreeMap::new(),
                severed: BTreeMap::new(),
            },
            ctx,
            return_ctxs: vec![ReturnCtx::Boundary],
            inline_stack: Vec::new(),
            expansion_frame_bases: Vec::new(),
            expansion_params: Vec::new(),
            seed_read: false,
            len_deferrals: Vec::new(),
            cur_stmt: None,
            cur_stmt_line: None,
            loop_depth: 0,
            fn_defs: BTreeMap::new(),
            call_fns: BTreeMap::new(),
            env_faces: BTreeMap::new(),
            env_face_depths: BTreeMap::new(),
            closure_fields: BTreeMap::new(),
            expansion_envs: Vec::new(),
            env_overrides: Vec::new(),
            instance_stack: Vec::new(),
            expansion_break_bases: Vec::new(),
            cond_depth: 0,
            ffi_verbs: BTreeMap::new(),
            builtin_verbs: BTreeMap::new(),
            ffi_mints: BTreeMap::new(),
            cdef_registry: BTreeMap::new(),
            mint_caps: BTreeMap::new(),
            index_base,
            instances: BTreeMap::from([(
                InstanceId::ROOT,
                InstanceInfo {
                    call: None,
                    fn_node: None,
                    params: Vec::new(),
                    body: Vec::new(),
                    frame_base: 0,
                    break_base: 0,
                    env_face: None,
                    ret: None,
                    children: BTreeMap::new(),
                    creation_envs: BTreeMap::new(),
                    closure_calls: BTreeSet::new(),
                    index_key_lanes: BTreeMap::new(),
                    access_elems: BTreeMap::new(),
                },
            )]),
            local_types: BTreeMap::new(),
            bare_sites: BTreeMap::new(),
            boundary_elem: None,
            diagnostics: Vec::new(),
            stmt_position_call: false,
        }
    }

    pub(super) fn snapshot_pre(
        &mut self,
        walk: &WalkState,
        severed: &BTreeMap<String, BTreeSet<usize>>,
    ) {
        self.snap = snap_of(walk, severed);
    }
    pub(super) fn snapshot_post(
        &mut self,
        walk: &WalkState,
        severed: &BTreeMap<String, BTreeSet<usize>>,
    ) {
        self.decl_snap = snap_of(walk, severed);
    }

    fn elem_of(&self, lat: &mut UnknownStore, ctor: &Expr) -> Ty {
        let id = self.ctx.sites[&ctor.id()];
        lat.resolve(&self.ctx.elems[id].clone())
    }

    fn declare_fn_binding(&mut self, name: &str, ptr: Option<NodeId>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.fns.insert(name.to_string(), ptr);
            // the binding is BORN at this frame — the burial law reads
            // the birthplace to tell a scope-correct death (a body or
            // block local dying with its frame) from an orphan (a face
            // outliving the binding's home)
            frame.born.insert(name.to_string());
            omni::checker_fn_write(name, omni::FnHome::Frame(self.frames.len() - 1), ptr, "declare");
        } else {
            // no frame home: the face takes the rider's ROOT home —
            // the checker-side twin of the walk's boundary-root scope
            // (d0). Top-level fn faces live here on the checker's own
            // statement-order timeline; the master snapshot no longer
            // has to carry them (phase 2 closed the gap the omni
            // layer's DROPPED receipt named).
            self.root_fns.insert(name.to_string(), ptr);
            omni::checker_fn_write(name, omni::FnHome::Root, ptr, "declare");
        }
    }

    fn assign_fn_binding(&mut self, name: &str, ptr: Option<NodeId>) {
        if let Some(depth) = self.frames.iter().rposition(|f| f.fns.contains_key(name)) {
            self.frames[depth].fns.insert(name.to_string(), ptr);
            omni::checker_fn_write(name, omni::FnHome::Frame(depth), ptr, "assign");
            return;
        }
        // The root home: an assignment to a root fn-bound name
        // rebinds the root binding's face — the checker-side mirror
        // of the walk's `binding_at_mut` stamp. Masked contexts (an
        // if arm or a loop body — `cond_depth`) keep their writes
        // local: the judge's law says an arm's or a latch's fn write
        // dies with its scope, so the root face must not move from
        // inside one.
        if self.cond_depth == 0
            && let Some(home) = self.root_fns.get_mut(name)
        {
            *home = ptr;
            omni::checker_fn_write(name, omni::FnHome::Root, ptr, "assign → root home");
            return;
        }
        omni::checker_fn_write(name, omni::FnHome::None, ptr, "assign — no home depth, dropped");
    }

    fn resolve_fn_binding(&self, name: &str) -> Option<Option<NodeId>> {
        let base = self.expansion_frame_bases.last().copied().unwrap_or(0);
        // The tiers in their original or_else order — the tier
        // that ANSWERS is the diagnosis the omni layer rides on. The
        // root home sits between the inline chain and the master
        // snapshot: the chain is the DEFINITION-SITE truth (inner
        // wins — a body's shadowing decl must beat the root face),
        // while the root home is the rider's own foundation for
        // top-level fn faces — the names the chain excludes
        // (CHAIN-FREEZE keeps root fn-bound names out of it). So a
        // top-level call resolves at its binding point's state — the
        // checker's statement-order timeline — instead of the master
        // snapshot's defcollection-dragged future.
        let mut tier = omni::Tier::Frames;
        let mut answer = self
            .frames[base..]
            .iter()
            .rev()
            .find_map(|f| f.fns.get(name).copied());
        if answer.is_none() {
            tier = omni::Tier::InlineChain;
            answer = self.inline_stack.last().and_then(|cur| {
                self.ctx
                    .fn_def_sites
                    .get(cur)
                    .and_then(|chain| chain.get(name))
                    .copied()
            });
        }
        if answer.is_none() {
            tier = omni::Tier::RootHome;
            answer = self.root_fns.get(name).copied();
        }
        if answer.is_none() {
            tier = omni::Tier::MasterSnap;
            answer = self.snap.fns.get(name).copied();
        }
        if omni::enabled() {
            omni::resolve_fn(
                name,
                tier,
                answer,
                &self.omni_frames_digest(base),
            );
        }
        answer
    }

    /// The rider frames' fn-face inventory, compact — the resolve
    /// event's "why did the tier answer" digest. An empty digest at
    /// top level sends the resolve to the root home and then the
    /// master snapshot; the root home's inventory rides after the
    /// frames so the answer's home is always named.
    fn omni_frames_digest(&self, base: usize) -> String {
        let mut s = String::new();
        for (i, frame) in self.frames[base..].iter().enumerate() {
            if frame.fns.is_empty() {
                continue;
            }
            s.push_str(if s.is_empty() { "[" } else { " | " });
            s.push_str(&format!("d{}{{", base + i));
            let mut first = true;
            for (n, ptr) in &frame.fns {
                if !first {
                    s.push_str(", ");
                }
                first = false;
                s.push_str(&match ptr {
                    Some(p) => format!("{n}=fn#{}", p.0),
                    None => format!("{n}=—"),
                });
            }
            s.push('}');
        }
        if !self.root_fns.is_empty() {
            s.push_str(if s.is_empty() { "[" } else { " | " });
            s.push_str("root{");
            let mut first = true;
            let shown = self.root_fns.len().min(6);
            for (i, (n, ptr)) in self.root_fns.iter().enumerate() {
                if i == shown {
                    s.push_str(&format!("…+{}", self.root_fns.len() - shown));
                    break;
                }
                if !first {
                    s.push_str(", ");
                }
                first = false;
                s.push_str(&match ptr {
                    Some(p) => format!("{n}=fn#{}", p.0),
                    None => format!("{n}=—"),
                });
            }
            s.push('}');
        }
        if s.is_empty() {
            "∅ (no rider frame carries a fn face)".to_string()
        } else {
            s + "]"
        }
    }

    fn resolves_to_fn(&self, name: &str) -> bool {
        self.resolve_fn_binding(name).flatten().is_some()
    }

    fn bind_function(&mut self, lat: &mut UnknownStore, name: &str, expr: &Expr, rebind: bool) -> Ty {
        let Expr::Function { params, body, .. } = expr else {
            unreachable!("bind_function on a non-function");
        };
        let ret = Ty::Unknown(lat.ensure_fn_ret(expr.id()));
        self.fn_defs.insert(
            expr.id(),
            FnDef {
                params: params.clone(),
                body: body.to_vec(),
                ret: ret.clone(),
                signature: None,
            },
        );
        self.collect_nested_fn_defs(lat, body);
        // The face's birthplace follows the statement form. A decl
        // mints a FRESH binding — the face is born at the innermost
        // scope (or the root home at top level). An assignment
        // REBINDS an existing binding — the face moves where the
        // binding lives: the write-through finds the home (a frame's
        // fns entry, else the root home) and stamps it there. Masked
        // contexts (an if arm or a loop body — `cond_depth`) and the
        // bound-nowhere placeholder keep the declare semantics: the
        // judge's law says an arm's or a latch's fn write dies with
        // its scope, and a name bound nowhere takes the innermost
        // placeholder home exactly as the walk's pre-bind does.
        let home_exists = self.frames.iter().any(|f| f.fns.contains_key(name))
            || self.root_fns.contains_key(name);
        if rebind && home_exists && self.cond_depth == 0 {
            self.assign_fn_binding(name, Some(expr.id()));
        } else {
            self.declare_fn_binding(name, Some(expr.id()));
        }
        Ty::Unknown(lat.ensure_fn_value(expr.id()))
    }

    fn collect_nested_fn_defs(&mut self, lat: &mut UnknownStore, body: &[Stmt]) {
        for stmt in body {
            match stmt {
                Stmt::Assignment { expr, .. } => {
                    if let Expr::Function { params, body, .. } = expr {
                        self.register_collected_fn(lat, expr, params, body);
                    }
                }
                Stmt::LocalDecl { exprs, .. } => {
                    for expr in exprs {
                        if let Expr::Function { params, body, .. } = expr {
                            self.register_collected_fn(lat, expr, params, body);
                        }
                    }
                }
                Stmt::If {
                    then_body,
                    else_body,
                    ..
                } => {
                    self.collect_nested_fn_defs(lat, then_body);
                    self.collect_nested_fn_defs(lat, else_body);
                }
                Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                    self.collect_nested_fn_defs(lat, body);
                }
                // these statements own no nested fn declarations at
                // their top level, and deeper expressions are not
                // scanned: a function literal anywhere but the whole
                // value of an Assignment or a LocalDecl expr is not a
                // declaration this collection claims
                Stmt::IndexAssign { .. }
                | Stmt::Print { .. }
                | Stmt::Expr { .. }
                | Stmt::Return { .. }
                | Stmt::Break { .. } => {}
            }
        }
    }

    fn register_collected_fn(
        &mut self,
        lat: &mut UnknownStore,
        expr: &Expr,
        params: &[String],
        body: &[Stmt],
    ) {
        let ptr = expr.id();
        if self.fn_defs.contains_key(&ptr) {
            return;
        }
        let ret = Ty::Unknown(lat.ensure_fn_ret(ptr));
        self.fn_defs.insert(
            ptr,
            FnDef {
                params: params.to_vec(),
                body: body.to_vec(),
                ret,
                signature: None,
            },
        );
        self.collect_nested_fn_defs(lat, body);
    }

    fn push_return_ctx(&mut self, expected: Ty) {
        self.return_ctxs.push(ReturnCtx::Internal(expected));
    }

    pub(super) fn finish(&mut self, lat: &mut UnknownStore) {
        self.finalize_boundary_elem(lat);
        self.resolve_recorded_facts(lat);
        self.replay_len_deferrals(lat);
    }

    pub(super) fn into_facts(self) -> CheckFacts {
        let Self {
            local_types,
            fn_defs,
            call_fns,
            env_faces,
            env_face_depths,
            closure_fields,
            ffi_verbs,
            builtin_verbs,
            ffi_mints,
            instances,
            boundary_elem,
            diagnostics,
            ctx,
            ..
        } = self;
        CheckFacts {
            local_types,
            fn_defs,
            call_fns,
            env_faces,
            env_face_depths,
            fn_capture_depths: ctx.fn_capture_depths,
            closure_fields,
            ffi_verbs,
            builtin_verbs,
            ffi_mints,
            instances,
            boundary_elem,
            diagnostics,
        }
    }

    fn replay_len_deferrals(&mut self, lat: &mut UnknownStore) {
        for (ty, line) in std::mem::take(&mut self.len_deferrals) {
            match lat.resolve(&ty) {
                Ty::Tbl(_) | Ty::Str | Ty::Unknown(_) => {}
                Ty::Any => {
                    signal!(trace::TRACE_CHK_ANY_LEN);
                }
                other => {
                    self.diagnostics.push(
                        Diagnostic::ty(format!(
                            "'#' requires a Table, String, or Any operand, got {}",
                            type_name(&other)
                        ))
                        .with_site(line),
                    );
                }
            }
        }
    }

    fn finalize_boundary_elem(&mut self, lat: &mut UnknownStore) {
        let id = lat.boundary_seed();
        let elem = match lat.resolve(&Ty::Unknown(id)) {
            Ty::Unknown(head) => {
                if !self.seed_read {
                    return;
                }
                signal!(trace::TRACE_BOUNDARY_ELEM_ANY);
                lat.bind(head, Ty::Any).ok();
                self.boundary_elem = Some(Ty::Any);
                self.resolve_fn_defs_through_subs(lat);
                return;
            }
            Ty::Tbl(_) => {
                self.diagnostics.push(Diagnostic::ty(
                    "the boundary 'arg' table's cells hold scalars — \
                     a table element cannot be inferred from 'arg[i]' usage",
                ));
                return;
            }
            Ty::Any => {
                signal!(trace::TRACE_BOUNDARY_ELEM_ANY);
                self.boundary_elem = Some(Ty::Any);
                self.resolve_fn_defs_through_subs(lat);
                return;
            }
            concrete => concrete,
        };
        signal!(trace::TRACE_BOUNDARY_ELEM_PINNED);
        self.boundary_elem = Some(elem);
        self.resolve_fn_defs_through_subs(lat);
    }

    fn resolve_fn_defs_through_subs(&mut self, lat: &mut UnknownStore) {
        for def in self.fn_defs.values_mut() {
            def.ret = lat.resolve(&def.ret);
            if let Some(sig) = &mut def.signature {
                for ty in sig.iter_mut() {
                    *ty = lat.resolve(ty);
                }
            }
        }
        for rec in self.instances.values_mut() {
            if let Some(ret) = &mut rec.ret {
                *ret = lat.resolve(ret);
            }
            if let Some(face) = &mut rec.env_face {
                for ty in face.values_mut() {
                    *ty = lat.resolve(ty);
                }
            }
        }
        for face in self.env_faces.values_mut() {
            for ty in face.values_mut() {
                *ty = lat.resolve(ty);
            }
        }
    }

    fn resolve_recorded_facts(&mut self, lat: &mut UnknownStore) {
        for ((stmt, instance, name), id) in std::mem::take(&mut self.bare_sites) {
            let resolved = lat.resolve(&Ty::Unknown(id));
            self.local_types.insert((stmt, instance, name), resolved);
        }
        for rec in self.instances.values_mut() {
            let resolved: Vec<(NodeId, Ty)> = rec
                .access_elems
                .iter()
                .map(|(&k, v)| (k, lat.resolve(v)))
                .collect();
            for (k, v) in resolved {
                rec.access_elems.insert(k, v);
            }
        }
    }
}
