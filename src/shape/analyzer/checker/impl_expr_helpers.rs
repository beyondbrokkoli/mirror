impl Checker {

    pub(super) fn check_block(&mut self, lat: &mut UnknownStore, stmts: &[Stmt]) {
        for stmt in stmts {
            if let Err(msg) = self.check_stmt(lat, stmt) {
                signal!(trace::TRACE_GHOST_BAIL_CHECKER);
                self.diagnostics
                    .push(msg.with_site(Some(stmt.loc().line as usize)));
                return;
            }
        }
    }

    fn numeric_operand(
        &mut self,
        lat: &mut UnknownStore,
        l: &Ty,
        r: &Ty,
        op: &BinOp,
        l_expr: &Expr,
        r_expr: &Expr,
    ) -> Result<Ty, Diagnostic> {
        let (l, r) = self.infer_numeric_pair(lat, l, r)?;
        match (&l, &r) {
            (Ty::Int, Ty::Int) => Ok(Ty::Int),
            (Ty::Flt, Ty::Flt) => Ok(Ty::Flt),
            (Ty::Int, Ty::Flt) | (Ty::Flt, Ty::Int) => Err(Diagnostic::ty(format!(
                "'{}' does not support mixed Integer and Float operands",
                bin_op_name(op)
            ))),
            _ => {
                let note = self
                    .any_provenance_note(lat, &l, l_expr)
                    .or_else(|| self.any_provenance_note(lat, &r, r_expr));
                Err(Diagnostic::ty(format!(
                    "'{}' requires numeric operands{}",
                    bin_op_name(op),
                    note.unwrap_or_default()
                )))
            }
        }
    }

    fn is_boundary_chained(&self, lat: &UnknownStore, ty: &Ty) -> bool {
        let seed = lat.boundary_seed();
        matches!(
            (
                lat.resolve(ty),
                lat.resolve(&Ty::Unknown(seed)),
            ),
            (Ty::Unknown(a), Ty::Unknown(b)) if a == b
        )
    }

    fn pin_bool(&mut self, lat: &mut UnknownStore, ty: &Ty) -> Result<Ty, Diagnostic> {
        let resolved = lat.resolve(ty);
        if let Ty::Unknown(id) = resolved {
            lat.bind(id, Ty::Bool)?;
            return Ok(Ty::Bool);
        }
        Ok(resolved)
    }

    fn infer_numeric_pair(
        &mut self,
        lat: &mut UnknownStore,
        l: &Ty,
        r: &Ty,
    ) -> Result<(Ty, Ty), Diagnostic> {
        let lr = lat.resolve(l);
        let rr = lat.resolve(r);
        let num = |t: &Ty| matches!(t, Ty::Int | Ty::Flt);
        let unknown_id = |t: &Ty| match t {
            Ty::Unknown(id) => *id,
            _ => unreachable!("the pin guards only reach unknowns"),
        };
        let unknown = |t: &Ty| matches!(t, Ty::Unknown(_));
        match (&lr, &rr) {
            (a, b) if num(a) && num(b) => Ok((lr.clone(), rr.clone())),
            (a, b) if num(a) && unknown(b) => {
                lat.bind(unknown_id(b), a.clone())?;
                Ok((a.clone(), a.clone()))
            }
            (a, b) if num(b) && unknown(a) => {
                lat.bind(unknown_id(a), b.clone())?;
                Ok((b.clone(), b.clone()))
            }
            (a, b) if unknown(a) && unknown(b) => {
                lat.bind(unknown_id(a), Ty::Int)?;
                lat.bind(unknown_id(b), Ty::Int)?;
                Ok((Ty::Int, Ty::Int))
            }
            _ => Ok((lr, rr)),
        }
    }

    fn pin_scalar_pair(
        &mut self,
        lat: &mut UnknownStore,
        l: &Ty,
        r: &Ty,
    ) -> Result<(), Diagnostic> {
        let lr = lat.resolve(l);
        let rr = lat.resolve(r);
        let scalar = |t: &Ty| matches!(t, Ty::Int | Ty::Flt | Ty::Bool | Ty::Str);
        match (&lr, &rr) {
            (a, Ty::Unknown(id)) if scalar(a) => {
                lat.bind(*id, a.clone())?;
            }
            (Ty::Unknown(id), b) if scalar(b) => {
                lat.bind(*id, b.clone())?;
            }
            _ => {}
        }
        Ok(())
    }

    fn any_provenance_note(&self, lat: &UnknownStore, ty: &Ty, expr: &Expr) -> Option<String> {
        if !matches!(lat.resolve(ty), Ty::Any) {
            return None;
        }
        // C: only a plain name or a direct index off one names a value
        // whose provenance a note could adopt
        let name = match expr {
            Expr::Identifier { name: n, .. } => n,
            Expr::Index { obj, .. } => match obj.as_ref() {
                Expr::Identifier { name: n, .. } => n,
                // C: a computed or nested base names nothing adoptable
                _ => return None,
            },
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
            | Expr::CoroModule { .. }
            | Expr::Field { .. }
            | Expr::Function { .. }
            | Expr::Call { .. } => return None,
        };
        let line = self.ctx.adopt_lines.get(name)?;
        signal!(trace::TRACE_CHK_ANY_PROVENANCE);
        Some(format!(
            " — '{name}' went dynamic at line {line} (the scalar-kind mix on its cells; \
             adoption is name-global)"
        ))
    }
}
