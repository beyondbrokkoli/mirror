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
        match lane {
            KeyLane::Str => {
                let TypedReg { reg, .. } = self.lower_expr(key, None)?;
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
                // The seam: every non-Const int key translates to
                // physical space exactly once, here. A bare integer
                // key folds the shift into one LoadInt in every base
                // (identical emission at base 0); a folded arithmetic
                // constant folds only when a shift applies, so base-0
                // emission stays byte-stable; everything else lowers
                // as written and subtracts the base when it is not 0.
                let base = self.shape.index_base;
                if (matches!(key, Expr::Integer { .. }) || base != 0)
                    && let Some(v) = crate::shape::const_key_value(key, base)
                {
                    let r = self.next_reg();
                    self.emit(Instruction::LoadInt {
                        target: Reg::new(r),
                        val: v,
                    });
                    return Ok(TableKey::Int(Reg::new(r)));
                }
                let TypedReg { reg, .. } = self.lower_expr(key, None)?;
                let reg = int_of(reg)?;
                if base == 0 {
                    return Ok(TableKey::Int(reg));
                }
                let c = self.next_reg();
                self.emit(Instruction::LoadInt {
                    target: Reg::new(c),
                    val: base,
                });
                let r = self.next_reg();
                self.num3(Instruction::Sub, r, reg, Reg::new(c));
                Ok(TableKey::Int(Reg::new(r)))
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
