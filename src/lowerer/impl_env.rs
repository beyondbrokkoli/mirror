impl<'a> IrLowerer<'a> {

                            fn mint_env(&mut self, ptr: NodeId) -> Result<Reg<Ptr>, LowerError> {
        let faces: BTreeMap<String, Ty> =
            self.shape.env_faces.get(&ptr).cloned().unwrap_or_default();
        let depths: BTreeMap<String, usize> = self
            .shape
            .env_face_depths
            .get(&ptr)
            .cloned()
            .unwrap_or_default();
        let stmt = self
            .cur_stmt
            .ok_or_else(|| LowerError(Diagnostic::lower("an env mint outside a statement")))?;
        // One env per set of per-name-agreeing faces. A memo matches
        // when every name the face and the memo SHARE resolves to the
        // same binding — same name AND same walk-scope depth. A name
        // held by only one side never conflicts, so disjoint faces
        // still share one env (cap_sibling_disjoint_faces) and the
        // same-face shape keeps its shared cell
        // (escape_env_cell_write_through). Same name, different depth
        // is a different binding — the shadow — and mints its own env
        // instead of absorbing its predecessor's cells (Closure
        // Finding A). The memo persists for the whole statement, so a
        // third creation read that agrees still shares rather than
        // forking a fresh env the way the old take-once memo forced.
        let hit = self.env_memos.iter().position(|memo| {
            memo.stmt == stmt
                && faces.keys().all(|n| {
                    !memo.depths.contains_key(n) || memo.depths.get(n) == depths.get(n)
                })
        });
        if let Some(idx) = hit {
            let memo = &self.env_memos[idx];
            let reg = memo.reg;
            let stored = memo.stored.clone();
            self.store_env_cells(reg, faces, &depths, stored, Some(idx))?;
            return Ok(reg);
        }
        let env_id = self.next_reg();
        let env = Reg::new(env_id);
        let flags = 0x01 | if faces.is_empty() { 0x00 } else { 0x80 };
        self.emit(Instruction::TableNew {
            target: env,
            elem: Ty::Str,
            flags,
        });
        self.env_regs.push(env);
        let mut stored = BTreeSet::new();
        for name in faces.keys() {
            stored.insert(name.clone());
        }
        self.store_env_cells(env, faces, &depths, BTreeSet::new(), None)?;
        self.env_memos.push(EnvMemo {
            stmt,
            reg: env,
            stored,
            depths,
        });
        Ok(env)
    }

    fn store_env_cells(
        &mut self,
        env: Reg<Ptr>,
        faces: BTreeMap<String, Ty>,
        depths: &BTreeMap<String, usize>,
        already: BTreeSet<String>,
        memo: Option<usize>,
    ) -> Result<(), LowerError> {
        for (name, ty) in faces {
            if already.contains(&name) {
                continue;
            }
            // The cell holds the FACE's binding, resolved at the
            // walk-scope depth the checker recorded — the innermost
            // same-named binding at the mint site is often a shadow
            // the face never captured (the depth-blind store was the
            // second half of Closure Finding A).
            let depth = match depths.get(&name) {
                Some(&depth) => depth,
                None => panic!(
                    "capture '{name}' has no recorded walk-scope depth \
                     in the face's depth map — the checker's face and \
                     the facts disagree on what the body captures: a \
                     structural desync, not a program error"
                ),
            };
            let local = self.read_var_at(&name, depth);
            let key = self.next_reg();
            self.emit(Instruction::StrKeyConst {
                target: Reg::new(key),
                val: alm_rt::rt::str_key_of_bytes(name.as_bytes()),
            });
            if matches!(ty, Ty::Tbl(_)) {
                let reg = ptr_of(local.reg)?;
                self.table_set(
                    env,
                    TableKey::Str(Reg::new(key)),
                    AnyReg::Ptr(reg),
                    &Ty::Str,
                )?;
            } else {
                let box_id = self.next_reg();
                let box_reg = Reg::new(box_id);
                self.emit(Instruction::TableNew {
                    target: box_reg,
                    elem: ty.clone(),
                    flags: 0,
                });
                let idx = self.next_reg();
                self.emit(Instruction::LoadInt {
                    target: Reg::new(idx),
                    val: 0,
                });
                self.table_set(box_reg, TableKey::Int(Reg::new(idx)), local.reg, &ty)?;
                self.table_set(
                    env,
                    TableKey::Str(Reg::new(key)),
                    AnyReg::Ptr(box_reg),
                    &Ty::Str,
                )?;
            }
            if let Some(idx) = memo {
                self.env_memos[idx].stored.insert(name.clone());
                self.env_memos[idx].depths.insert(name, depth);
            }
        }
        Ok(())
    }

                        fn enter_env_ctx(&mut self, env: Reg<Ptr>, ptr: NodeId) -> Result<(), LowerError> {
        let faces: BTreeMap<String, Ty> =
            self.shape.env_faces.get(&ptr).cloned().unwrap_or_default();
        for (name, ty) in faces.clone() {
            let key = self.next_reg();
            self.emit(Instruction::StrKeyConst {
                target: Reg::new(key),
                val: alm_rt::rt::str_key_of_bytes(name.as_bytes()),
            });
            let cell_id = self.next_reg();
            let cell = Reg::new(cell_id);
            self.emit(Instruction::TableGet(<Ptr as CellRepr>::get(
                cell,
                env,
                TableKey::Str(Reg::new(key)),
            )));
            if matches!(ty, Ty::Tbl(_)) {
                self.declare_var(name, AnyReg::Ptr(cell), ty, LayoutVerdict::Sparse);
            } else {
                let idx = self.next_reg();
                self.emit(Instruction::LoadInt {
                    target: Reg::new(idx),
                    val: 0,
                });
                let out = self.next_reg();
                let v = self.table_get(out, cell, TableKey::Int(Reg::new(idx)), &ty);
                self.declare_var(name, v, ty, LayoutVerdict::default());
            }
        }
        self.env_ctxs.push(EnvCtx { reg: env, faces });
        Ok(())
    }

            fn env_write_back(&mut self, name: &str, value: AnyReg) -> Result<(), LowerError> {
        let Some(ctx) = self.env_ctxs.last().cloned() else {
            return Ok(());
        };
        let Some(ty) = ctx.faces.get(name).cloned() else {
            return Ok(());
        };
        if matches!(ty, Ty::Tbl(_)) {
            return Ok(());
        }
        let key = self.next_reg();
        self.emit(Instruction::StrKeyConst {
            target: Reg::new(key),
            val: alm_rt::rt::str_key_of_bytes(name.as_bytes()),
        });
        let cell_id = self.next_reg();
        let cell = Reg::new(cell_id);
        self.emit(Instruction::TableGet(<Ptr as CellRepr>::get(
            cell,
            ctx.reg,
            TableKey::Str(Reg::new(key)),
        )));
        let idx = self.next_reg();
        self.emit(Instruction::LoadInt {
            target: Reg::new(idx),
            val: 0,
        });
        self.table_set(cell, TableKey::Int(Reg::new(idx)), value, &ty)?;
        Ok(())
    }
}
