impl Checker {

    fn check_condition(
        &mut self,
        lat: &mut UnknownStore,
        condition: &Expr,
        kw: &str,
    ) -> Result<(), Diagnostic> {
        let ty = self.check_expr(lat, condition)?;
        let ty = self.pin_bool(lat, &ty)?;
        if ty != Ty::Bool {
            let note = self.any_provenance_note(lat, &ty, condition);
            return Err(Diagnostic::ty(format!(
                "'{}' condition must be a Boolean, got {}{}",
                kw,
                type_name(&ty),
                note.unwrap_or_default()
            )));
        }
        Ok(())
    }

    fn create_closure(
        &mut self,
        lat: &mut UnknownStore,
        expr: &Expr,
        name: &str,
        ptr: NodeId,
    ) -> Result<Ty, Diagnostic> {
        if self.inline_stack.contains(&ptr) {
            return Err(Diagnostic::ty(format!(
                "'{name}' cannot escape its own expansion — \
                 a closure cannot capture itself"
            )));
        }
        if self.cond_depth > 0 {
            return Err(Diagnostic::ty(format!(
                "'{name}' cannot be created inside a conditional arm \
                 or loop body — creations are straight-line (V1)"
            )));
        }
        let caps = self.ctx.fn_captures.get(&ptr).cloned().unwrap_or_default();
        let mut face: BTreeMap<String, Ty> = BTreeMap::new();
        for cap in &caps {
            let ty = self.var_type(lat, cap)?;
            face.insert(cap.clone(), lat.resolve(&ty));
        }
        match self.env_faces.get(&ptr) {
            None => {
                self.env_faces.insert(ptr, face);
            }
            Some(prev) => {
                for (n, t) in &face {
                    match prev.get(n) {
                        Some(p) => {
                            lat.unify(p, t)?;
                        }
                        None => {
                            panic!(
                                "'{name}' has no capture '{n}' at an earlier \
                                 creation site — captures are fixed body \
                                 text: one body, one face set, and \
                                 env_faces is inserted before any \
                                 Ty::Closure escapes",
                            );
                        }
                    }
                }
            }
        }
        self.shadow_pair().creation_envs.insert(expr.id(), ptr);
        Ok(Ty::Closure(ptr))
    }

    fn reject_face_extraction(expr: &Expr, ty: &Ty) -> Result<(), Diagnostic> {
        if matches!(expr, Expr::Field { .. }) && matches!(ty, Ty::Closure(_)) {
            return Err(Diagnostic::ty(
                "the face rides the field — a closure field cannot be \
                 extracted into a name; call it through the record",
            ));
        }
        Ok(())
    }

    fn closure_field_fn(&self, obj: &Expr, field: &str) -> Option<NodeId> {
        let Expr::Identifier { name: n, .. } = obj else {
            return None;
        };
        let bind = self.snap.binds.get(n)?;
        for s in bind.aliases.iter() {
            if !is_root(s)
                && !is_ghost(s)
                && let Some(ptr) = self.closure_fields.get(&(*s, field.to_string()))
            {
                return Some(*ptr);
            }
        }
        None
    }

    fn carries_closure(&self, expr: &Expr) -> bool {
        if let Expr::Identifier { name: n, .. } = expr
            && let Some(bind) = self.snap.binds.get(n)
        {
            return bind.aliases.iter().any(|s| {
                !is_root(s) && !is_ghost(s) && self.closure_fields.keys().any(|(site, _)| site == s)
            });
        }
        false
    }
}
