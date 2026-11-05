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
}
