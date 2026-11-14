impl Checker {
    pub(super) fn new(ctx: CheckCtx) -> Self {
        let index_base = ctx.index_base;
        Self {
            frames: Vec::new(),
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
        }
    }

    fn assign_fn_binding(&mut self, name: &str, ptr: Option<NodeId>) {
        let Some(depth) = self.frames.iter().rposition(|f| f.fns.contains_key(name)) else {
            return;
        };
        self.frames[depth].fns.insert(name.to_string(), ptr);
    }

    fn resolve_fn_binding(&self, name: &str) -> Option<Option<NodeId>> {
        let base = self.expansion_frame_bases.last().copied().unwrap_or(0);
        self.frames[base..]
            .iter()
            .rev()
            .find_map(|f| f.fns.get(name).copied())
            .or_else(|| {
                self.inline_stack
                    .last()
                    .and_then(|&cur| self.ctx.fn_def_sites.get(&cur)?.get(name).copied())
            })
            .or_else(|| self.snap.fns.get(name).copied())
    }

    fn resolves_to_fn(&self, name: &str) -> bool {
        self.resolve_fn_binding(name).flatten().is_some()
    }

    fn bind_function(&mut self, lat: &mut UnknownStore, name: &str, expr: &Expr) -> Ty {
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
        self.declare_fn_binding(name, Some(expr.id()));
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
            ..
        } = self;
        CheckFacts {
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
