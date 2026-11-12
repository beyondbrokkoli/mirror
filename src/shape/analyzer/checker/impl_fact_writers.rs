impl Checker {

    fn shadow_pair(&mut self) -> &mut InstanceInfo {
        let owner = self
            .instance_stack
            .first()
            .copied()
            .unwrap_or(InstanceId::ROOT);
        self.instances
            .get_mut(&owner)
            .expect("ROOT seeded at construction; every mint records itself")
    }

    fn check_index_base(
        &mut self,
        lat: &mut UnknownStore,
        obj: &Expr,
    ) -> Result<(Ty, String), Diagnostic> {
        let ty = self.check_expr(lat, obj)?;
        match ty {
            Ty::Tbl(elem) => {
                let desc = type_name(&Ty::Tbl(elem.clone()));
                signal!(trace::TRACE_CHK_ELEM_FACE);
                self.shadow_pair()
                    .access_elems
                    .insert(obj.id(), (*elem).clone());
                Ok((*elem, desc))
            }
            _ => Err(Diagnostic::ty(format!(
                "cannot index {} — only tables support '[]'",
                type_name(&ty)
            ))),
        }
    }

    fn check_index_key(&mut self, lat: &mut UnknownStore, key: &Expr) -> Result<Ty, Diagnostic> {
        let key_ty = self.check_expr(lat, key)?;
        let key_ty = lat.resolve(&key_ty);
        let key_ty = if let Ty::Unknown(id) = key_ty {
            lat.bind(id, Ty::Int)?;
            Ty::Int
        } else {
            key_ty
        };
        if key_ty != Ty::Int && key_ty != Ty::Str {
            let note = self.any_provenance_note(lat, &key_ty, key);
            return Err(Diagnostic::ty(format!(
                "table index must be an Integer or a String, got {}{}",
                type_name(&key_ty),
                note.unwrap_or_default()
            )));
        }
        if key_ty == Ty::Str {
            signal!(trace::TRACE_CHK_STRKEY);
        }
        signal!(trace::TRACE_CHK_KEY_LANE);
        let lane = if key_ty == Ty::Str {
            KeyLane::Str
        } else {
            KeyLane::Int
        };
        self.shadow_pair().index_key_lanes.insert(key.id(), lane);
        Ok(key_ty)
    }

    fn record_verb(&mut self, call: &Expr, verb: FfiVerb) {
        signal!(trace::TRACE_CHK_FFI_VERB);
        self.ffi_verbs.insert(call.id(), verb);
    }

    /// Stamp the mint fact with the local name it binds (the
    /// second-writer contract is per-name) and track the name's
    /// declared capacity; a non-mint initializer clears the entry.
    fn track_mint_binding(&mut self, name: &str, expr: &Expr) {
        if matches!(
            self.ffi_verbs.get(&expr.id()),
            Some(FfiVerb::New { .. })
        ) && let Some(mint) = self.ffi_mints.get_mut(&expr.id())
        {
            mint.name = name.to_string();
            self.mint_caps.insert(name.to_string(), mint.capacity);
        } else {
            self.mint_caps.remove(name);
        }
    }

    /// The static-key bounds law: a literal (or checker-foldable) key
    /// beyond a mint's declared capacity rejects at compile time,
    /// naming the buffer and the capacity. Dynamic keys keep the
    /// normal table semantics — the capacity is a floor, not a
    /// ceiling (the documented divergence from C).
    fn check_mint_bounds(&self, obj: &Expr, key: &Expr) -> Result<(), Diagnostic> {
        let Expr::Identifier { name, .. } = obj else {
            return Ok(());
        };
        let Some(&cap) = self.mint_caps.get(name) else {
            return Ok(());
        };
        if let Some(phys) = const_key_value(key, self.index_base)
            && (phys < 0 || phys >= cap)
        {
            return Err(Diagnostic::bounds(format!(
                "'{name}' is a declared buffer of capacity {cap} — key {} is \
                 beyond it (ffi.new's capacity is a floor for dynamic keys, a \
                 wall for static ones)",
                phys + self.index_base
            )));
        }
        Ok(())
    }
}
