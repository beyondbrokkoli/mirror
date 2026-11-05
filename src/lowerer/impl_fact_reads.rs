impl<'a> IrLowerer<'a> {

                                                fn lower_index_key(&mut self, key: &Expr) -> Result<TableKey, LowerError> {
        if let Expr::String { value: s, .. } = key {
            let r = self.next_reg();
            self.emit(Instruction::StrKeyConst {
                target: Reg::new(r),
                val: alm_rt::rt::str_key_of_bytes(s.as_bytes()),
            });
            return Ok(TableKey::Str(Reg::new(r)));
        }
        let owner = self
            .instance_chain
            .first()
            .copied()
            .unwrap_or(InstanceId::ROOT);
        let lane = *self
            .shape
            .instances[&owner]
            .index_key_lanes
            .get(&key.id())
            .expect(
                "an index key carries no recorded lane — the checker \
                 records every dynamic key's lane on the OUTERMOST \
                 extent's record",
            );
        let TypedReg { reg, .. } = self.lower_expr(key, None)?;
        match lane {
            KeyLane::Str => {
                let s = str_lane(reg)?;
                let r = self.next_reg();
                self.emit(Instruction::StrKey {
                    target: Reg::new(r),
                    s,
                });
                signal!(trace::TRACE_LOWER_KEY_LANE);
                Ok(TableKey::Str(Reg::new(r)))
            }
            KeyLane::Int => {
                signal!(trace::TRACE_LOWER_KEY_LANE);
                Ok(TableKey::Int(int_of(reg)?))
            }
        }
    }

                    fn elem_face_of(&self, obj: &Expr) -> Result<Ty, LowerError> {
        let owner = self
            .instance_chain
            .first()
            .copied()
            .unwrap_or(InstanceId::ROOT);
        let elem = self
            .shape
            .instances[&owner]
            .access_elems
            .get(&obj.id())
            .cloned()
            .expect(
                "a table access carries no recorded elem face — the \
                 checker records every access base's elem face on the \
                 OUTERMOST extent's record",
            );
        signal!(trace::TRACE_LOWER_ELEM_FACE);
        Ok(elem)
    }
}
