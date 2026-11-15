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
            // Closure face-Ty law (the parked creation-read-site item):
            // the face's Ty was stamped at the creation-read site — the
            // innermost same-named binding THERE — while this store
            // reads the binding at the recorded walk-scope depth. A
            // same-name shadow of a different concrete type between the
            // two depths stamps the shadow's repr into the box and
            // stores the outer binding's register into it; the repr law
            // would panic mid-store. Die here instead, with the law
            // named. Table faces stay out (they alias by pointer, no
            // box element rides them), as does anything not fully
            // concrete — Unknown/Pending disagreements are the
            // constraint lattice's business, not a repr split.
            let face_repr = match &ty {
                Ty::Int | Ty::Flt | Ty::Bool | Ty::Str => Some(ty.clone()),
                _ => None,
            };
            let local_repr = match &local.ty {
                Ty::Int | Ty::Flt | Ty::Bool | Ty::Str => Some(local.ty.clone()),
                _ => None,
            };
            if let (Some(face), Some(stored)) = (&face_repr, &local_repr)
                && face != stored
            {
                // The same vocabulary the checker's diagnostics use.
                let name_of = |t: &Ty| match t {
                    Ty::Int => "Integer",
                    Ty::Flt => "Float",
                    Ty::Bool => "Boolean",
                    Ty::Str => "String",
                    _ => "?",
                };
                return Err(LowerError(Diagnostic::lower(format!(
                    "capture '{name}' stamps a {} face but its binding \
                     at the recorded walk-scope depth holds a {} — a \
                     same-name shadow between the creation-read site and \
                     the binding depth split the face (Closure face-Ty: \
                     the face must ride the binding depth)",
                    name_of(face),
                    name_of(stored)
                ))));
            }
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
            } else if let Some(cell) = local.cell {
                // Box-is-binding: the binding minted its ONE cell at
                // declaration — store the pointer, register this env
                // as a holder for the root drain. No snapshot, no
                // creation: the cell IS the binding.
                if let Some(holders) = self.binding_cells.get_mut(&cell.id) {
                    holders.push(env);
                }
                self.table_set(
                    env,
                    TableKey::Str(Reg::new(key)),
                    AnyReg::Ptr(cell),
                    &Ty::Str,
                )?;
            } else {
                // A capture whose binding the walk never boxed (the
                // misaligned surface: the mint resolves the recorded
                // depth to a binding this stack did not declare). The
                // legacy path — a fresh cell holding this register's
                // value — keeps today's behavior.
                let slot_id = self.scope_ids[depth];
                // The definer expansion's instance owns this binding's
                // slot identity — the fold-back's liveness proof.
                let inst = self
                    .instance_chain
                    .last()
                    .copied()
                    .unwrap_or(InstanceId::ROOT);
                self.instance_slots
                    .entry(inst)
                    .or_default()
                    .insert(name.clone(), slot_id);
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
                self.binding_cells.insert(box_id, vec![env]);
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
                self.declare_var(name, AnyReg::Ptr(cell), ty, LayoutVerdict::Sparse)?;
            } else {
                let idx = self.next_reg();
                self.emit(Instruction::LoadInt {
                    target: Reg::new(idx),
                    val: 0,
                });
                let out = self.next_reg();
                let v = self.table_get(out, cell, TableKey::Int(Reg::new(idx)), &ty);
                self.declare_var(name, v, ty, LayoutVerdict::default())?;
            }
        }
        let depths: BTreeMap<String, usize> = self
            .shape
            .env_face_depths
            .get(&ptr)
            .cloned()
            .unwrap_or_default();
        let scope_depth = self.scopes.len() - 1;
        self.env_ctxs.push(EnvCtx {
            reg: env,
            faces,
            depths,
            scope_depth,
        });
        Ok(())
    }

    /// Expansion-side write-back: an assignment to a face name inside
    /// its own closure's expansion writes THE binding's one shared
    /// cell through the callee's env (`env[name]` — runtime-exact,
    /// escaped closures included). Every other env over the binding
    /// holds the same cell pointer, so the sibling views and the
    /// definer's fold-back all read the written value — no routing
    /// table needed on this path.
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
        self.write_env_cell(ctx.reg, name, value, &ty)
    }

    /// One cell write through its env: `env[name]` → the box, `box[0]`
    /// ← value. Runtime-exact per env — no compile-time binding
    /// identity needed.
    fn write_env_cell(
        &mut self,
        env: Reg<Ptr>,
        name: &str,
        value: AnyReg,
        ty: &Ty,
    ) -> Result<(), LowerError> {
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
        let idx = self.next_reg();
        self.emit(Instruction::LoadInt {
            target: Reg::new(idx),
            val: 0,
        });
        self.table_set(cell, TableKey::Int(Reg::new(idx)), value, ty)
    }

    /// One cell read through its env: `env[name]` → the box, `box[0]`
    /// → value register.
    fn read_env_cell(&mut self, env: Reg<Ptr>, name: &str, ty: &Ty) -> AnyReg {
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
        let idx = self.next_reg();
        self.emit(Instruction::LoadInt {
            target: Reg::new(idx),
            val: 0,
        });
        let out = self.next_reg();
        self.table_get(out, cell, TableKey::Int(Reg::new(idx)), ty)
    }

    /// The pop fold-back. The expansion's face writes live in THE
    /// shared cell; the register-side views outside it re-read it.
    /// The definer's slot is located through the mint-time record:
    /// the definer expansion's instance holds the binding's slot id,
    /// and the instance chain proves the slot is still the live one —
    /// a call site outside the definer's lowering (an escaped
    /// closure) finds no instance and skips, which is correct: the
    /// dead register has no reader. Position must also equal the
    /// face's recorded walk depth, so a nested instance's same-named
    /// entry for a DIFFERENT binding never misfires. Outer expansions'
    /// face-local snapshots re-read their own env's cell — the same
    /// shared storage, so they are exact too.
    fn fold_back_env(&mut self, ctx: &EnvCtx) -> Result<(), LowerError> {
        for (name, ty) in &ctx.faces {
            if matches!(ty, Ty::Tbl(_)) {
                continue;
            }
            let v = self.read_env_cell(ctx.reg, name, ty);
            if let Some(&d) = ctx.depths.get(name) {
                // Depth 0: the boundary root is the base of every stack
                // and outlives every expansion — the fold always lands.
                // Deeper bindings are per-instance: the definer's slot
                // is only live while its expansion is on the chain (t3's
                // `b()` inside Make's body folds; a post-return call
                // skips the dead register, which has no reader).
                let mut target: Option<usize> = if d == 0 { Some(0) } else { None };
                if target.is_none() {
                    for inst in self.instance_chain.iter().rev() {
                        let Some(&slot_id) =
                            self.instance_slots.get(inst).and_then(|m| m.get(name))
                        else {
                            continue;
                        };
                        let Some(pos) = self.scope_ids.iter().position(|&id| id == slot_id)
                        else {
                            continue;
                        };
                        if pos == d {
                            target = Some(pos);
                            break;
                        }
                    }
                }
                if let Some(pos) = target {
                    self.update_var_at(name, pos, v, ty.clone(), LayoutVerdict::default())?;
                }
            }
            let outers: Vec<(usize, Reg<Ptr>, String)> = self
                .env_ctxs
                .iter()
                .filter(|c| c.faces.contains_key(name))
                .map(|c| (c.scope_depth, c.reg, name.clone()))
                .collect();
            for (d, ereg, n) in outers {
                let v2 = self.read_env_cell(ereg, &n, ty);
                self.update_var_at(&n, d, v2, ty.clone(), LayoutVerdict::default())?;
            }
        }
        Ok(())
    }
}
