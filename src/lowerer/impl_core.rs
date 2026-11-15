impl<'a> IrLowerer<'a> {
    pub fn new(shape: &'a ShapeFacts) -> Self {
        Self {
            blocks: vec![BasicBlock::new(0)],
            diagnostics: Vec::new(),
            current_block: 0,
            free_reg: 0,
            scopes: vec![BTreeMap::new()],
            reserves: Vec::new(),
            shape,
            cur_loc: None,
            handles: FreeHandles {
                site_regs: BTreeMap::new(),
            },
            ghost_keeps: BTreeMap::new(),
            if_tag_regs: BTreeMap::new(),
            call_tag_regs: BTreeMap::new(),
            ctrl_depth: 0,
            return_ctxs: Vec::new(),
            break_ctxs: Vec::new(),
            instance_chain: Vec::new(),
            event_cursor: BTreeMap::new(),
            env_memos: Vec::new(),
            env_regs: Vec::new(),
            binding_cells: BTreeMap::new(),
            fn_frames: Vec::new(),
            captured: shape
                .fn_capture_depths
                .values()
                .flat_map(|m| m.iter().map(|(n, &d)| (d, n.clone())))
                .collect(),
            scope_cells: vec![BTreeSet::new()],
            cur_stmt: None,
        }
    }

    /// Free a dying scope's own cells that nothing holds. A cell with
    /// env holders escapes (cap_escape_lifetime) — the root drain's
    /// keeps plan frees it through the last holder. An unheld cell has
    /// no reader after its scope dies: freeing here keeps the atexit
    /// leak check balanced without any scope-exit-free hazard.
    fn free_dead_scope_cells(&mut self, cells: &BTreeSet<RegId>) {
        let dead: Vec<Reg<Ptr>> = cells
            .iter()
            .filter(|id| {
                self.binding_cells
                    .get(id)
                    .is_some_and(|holders| holders.is_empty())
            })
            .map(|&id| Reg::<Ptr>::new(id))
            .collect();
        for cell in dead {
            self.binding_cells.remove(&cell.id);
            signal!(trace::TRACE_SCOPE_CELL_FREE);
            self.emit(Instruction::TableFree { table: cell });
        }
    }

    /// The boundary return's drain. The root drain rides the
    /// fall-through tail only, so a chunk leaving through a top-level
    /// `return` carries its own: the envs minted so far — in mint
    /// order, keeps scoped to THIS prefix, because a holder minted
    /// after the return never exists on this path — and every cell no
    /// minted env holds, which is the unheld cells plus the cells
    /// whose only holder envs mint later. Compile state stays intact:
    /// the tail drain still serves the fall-through path with the
    /// full inventory, and each path frees its own execution exactly
    /// once. V1 (creations are straight-line, transitively — the
    /// leak_contract/p11/p12 receipts) keeps every register here
    /// dominating this block.
    fn emit_return_path_drain(&mut self) {
        let envs = self.env_regs.clone();
        for (i, env) in envs.iter().enumerate() {
            let mut keeps: Vec<Reg<Ptr>> = Vec::new();
            for (&cell_id, holders) in &self.binding_cells {
                let held_here = holders.iter().any(|e| e.id == env.id);
                let held_later_prefix = holders.iter().any(|e| {
                    envs[i + 1..].iter().any(|later| later.id == e.id)
                });
                if held_here && held_later_prefix {
                    keeps.push(Reg::new(cell_id));
                }
            }
            if keeps.is_empty() {
                self.emit(Instruction::TableFree { table: *env });
            } else {
                self.emit(Instruction::TableFreeExcept {
                    table: *env,
                    keeps,
                });
            }
        }
        let orphans: Vec<Reg<Ptr>> = self
            .binding_cells
            .iter()
            .filter(|(_, holders)| {
                holders.is_empty()
                    || holders
                        .iter()
                        .all(|h| !envs.iter().any(|e| e.id == h.id))
            })
            .map(|(&id, _)| Reg::<Ptr>::new(id))
            .collect();
        for cell in orphans {
            self.emit(Instruction::TableFree { table: cell });
        }
    }

    /// The inline return's crossed-cell frees. The expansion's return
    /// edge exits the body's scopes, whose pop's free_dead_scope_cells
    /// emits into the dead continuation after the return terminated
    /// the block (the leak_contract/p4a receipt). Every cell minted in
    /// a crossed scope whose holders have not minted yet frees on THIS
    /// edge: unheld cells, and cells whose only holder envs mint later
    /// in the body — those envs never exist on this path. A cell held
    /// by an already-minted env escapes with it and stays with the
    /// root drain. Inventory is left to the pop, which still removes
    /// it for the tail; the two emissions ride exclusive paths and
    /// each execution frees exactly once.
    fn emit_crossed_cell_frees(&mut self) {
        let base = self.fn_frames.last().map(|&(base, _)| base).unwrap_or(0);
        let crossed: Vec<RegId> = self.scope_cells[base..]
            .iter()
            .flat_map(|set| set.iter().copied())
            .collect();
        for cell_id in crossed {
            let held_live = self.binding_cells.get(&cell_id).is_some_and(|holders| {
                holders
                    .iter()
                    .any(|h| self.env_regs.iter().any(|e| e.id == h.id))
            });
            if !held_live {
                self.emit(Instruction::TableFree {
                    table: Reg::<Ptr>::new(cell_id),
                });
            }
        }
    }

    /// The declare site's walk depth: the current fn body's walk
    /// depth plus the scope offset within the body. The walk and this
    /// stack push one scope per body/do/arm in lockstep, so the
    /// offset is exact on every surface — the box-on-declare gate's
    /// key, call-site independent by construction.
    fn cur_walk_depth(&self) -> usize {
        match self.fn_frames.last() {
            Some(&(base, walk_depth)) => walk_depth + (self.scopes.len() - 1 - base),
            None => self.scopes.len() - 1,
        }
    }

    fn new_block(&mut self) -> BlockId {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock::new(id));
        id
    }

    fn next_reg(&mut self) -> RegId {
        let r = self.free_reg;
        self.free_reg += 1;
        r
    }

    fn emit(&mut self, instr: Instruction) {
        let block = &mut self.blocks[self.current_block];
        block.locs.push(self.cur_loc);
        block.instrs.push(instr);
    }

    fn terminate(&mut self, term: Terminator) {
        let block = &mut self.blocks[self.current_block];
        block.term_loc = self.cur_loc;
        block.terminator = Some(term);
    }

    fn push_dbg_bind(&mut self, name: &str, reg: AnyReg, param: Option<u32>) {
        let after = self.blocks[self.current_block].instrs.len().checked_sub(1);
        if let Some(after) = after {
            self.blocks[self.current_block].dbg_values.push(DbgBind {
                after,
                name: name.to_string(),
                reg: reg.id(),
                kind: reg.kind(),
                loc: self.cur_loc,
                param,
            });
        }
    }

    fn in_block<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.scopes.push(BTreeMap::new());
        self.scope_cells.push(BTreeSet::new());
        self.ctrl_depth += 1;
        let r = f(self);
        self.ctrl_depth -= 1;
        let cells = self.scope_cells.pop().expect("index-aligned");
        self.free_dead_scope_cells(&cells);
        self.scopes.pop();
        r
    }

    fn at_depth<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.ctrl_depth += 1;
        let r = f(self);
        self.ctrl_depth -= 1;
        r
    }

    fn in_do<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.scopes.push(BTreeMap::new());
        self.scope_cells.push(BTreeSet::new());
        let r = f(self);
        let cells = self.scope_cells.pop().expect("index-aligned");
        self.free_dead_scope_cells(&cells);
        self.scopes.pop();
        r
    }

    fn emit_scope_frees(&mut self, key: (NodeId, u8)) -> Result<(), LowerError> {
        let frees = self
            .shape
            .do_exit_frees
            .get(&key)
            .cloned()
            .unwrap_or_default();
        self.emit_frees(&frees)?;
        if let Some(arms) = self.shape.arm_do_exit_frees.get(&key) {
            for e in arms {
                self.emit_arm_free(e)?;
            }
        }
        Ok(())
    }

    fn resolve_keeps(&mut self, keeps: &[Keep]) -> Result<Vec<Reg<Ptr>>, LowerError> {
        let mut regs = Vec::new();
        for k in keeps {
            match k {
                Keep::Name(n) => {
                    let local = self.read_var(n)?;
                    let reg = ptr_of(local.reg).expect(
                        "a borrower planned as a keep holds a non-pointer \
                         register — the analyzer plans keeps only over \
                         pointer-carrying names",
                    );
                    regs.push(reg);
                }
                Keep::Ghost(g) => {
                    let &r = self.ghost_keeps.get(g).expect(
                        "a housed row's keep register was never captured — \
                         the analyzer captures every housed row's ghost \
                         keep at mint time",
                    );
                    regs.push(r);
                }
            }
        }
        regs.sort_unstable_by_key(|r| r.id);
        regs.dedup_by_key(|r| r.id);
        Ok(regs)
    }

    fn emit_arm_free(&mut self, e: &ArmFreeEntry) -> Result<(), LowerError> {
        let tag = match e.tag {
            TagSrc::If(stmt) => self.if_tag_regs.get(&stmt).copied(),
            TagSrc::Call(expr) => self.call_tag_regs.get(&expr).copied(),
        }
        .expect(
            "a mixed join's tag phi never lowered — every If/Call tag \
             in an arm-free's extent lowers before the join",
        );
        let carrier = self.read_var(&e.carrier)?;
        let table = ptr_of(carrier.reg)?;
        let keep_regs = self.resolve_keeps(&e.keeps)?;
        if keep_regs.iter().any(|k| k.id == table.id) {
            return Ok(());
        }
        let c_reg = self.next_reg();
        self.emit(Instruction::LoadBool {
            target: Reg::new(c_reg),
            val: e.nothing_on,
        });
        let eq_reg = self.next_reg();
        self.cmp3(Instruction::Eq, eq_reg, tag, Reg::new(c_reg));
        let nothing_block = self.new_block();
        let free_block = self.new_block();
        let cont = self.new_block();
        self.terminate(Terminator::Branch {
            cond: Reg::new(eq_reg),
            true_block: nothing_block,
            false_block: free_block,
        });
        self.current_block = free_block;
        signal!(trace::TRACE_JOIN_ARM_FREE);
        if keep_regs.is_empty() {
            self.emit(Instruction::TableFree { table });
        } else {
            signal!(trace::TRACE_DO_EXIT_KEEP_FREE);
            self.emit(Instruction::TableFreeExcept {
                table,
                keeps: keep_regs,
            });
        }
        self.terminate(Terminator::Jump(cont));
        self.current_block = nothing_block;
        self.terminate(Terminator::Jump(cont));
        self.current_block = cont;
        Ok(())
    }

    fn emit_frees(&mut self, frees: &[DoExitFree]) -> Result<(), LowerError> {
        let mut plain: Vec<Reg<Ptr>> = Vec::new();
        let mut kept: Vec<(Reg<Ptr>, Vec<Reg<Ptr>>)> = Vec::new();
        for f in frees {
            let carrier = self.read_var(&f.carrier)?;
            if matches!(carrier.ty, Ty::Closure(_)) {
                panic!(
                    "a closure value's env has one owner (its mint) — '{}' \
                     carries a face; a face read never feeds a binding \
                     (the Extraction Law)",
                    f.carrier
                );
            }
            let table = ptr_of(carrier.reg)?;
            signal!(trace::TRACE_DO_EXIT_FREE_PHI);
            if f.keeps.is_empty() {
                plain.push(table);
            } else {
                kept.push((table, self.resolve_keeps(&f.keeps)?));
            }
        }

        plain.sort_unstable_by_key(|reg| reg.id);
        plain.dedup_by_key(|reg| reg.id);
        let mut seen: BTreeSet<RegId> = BTreeSet::new();
        for (table, keeps) in kept {
            if keeps.iter().any(|k| k.id == table.id) || !seen.insert(table.id) {
                continue;
            }
            signal!(trace::TRACE_DO_EXIT_KEEP_FREE);
            self.emit(Instruction::TableFreeExcept { table, keeps });
        }
        for reg in plain.into_iter().rev() {
            if !seen.insert(reg.id) {
                continue;
            }
            self.emit(Instruction::TableFree { table: reg });
        }
        Ok(())
    }

    fn emit_root_frees(&mut self) -> Result<(), LowerError> {
        for f in &self.shape.root_exit_frees {
            for k in &f.keeps {
                if let Keep::Name(n) = k {
                    let local = self.read_var(n)?;
                    let reg = ptr_of(local.reg).expect(
                        "a borrower planned as a keep holds a non-pointer \
                         register — the analyzer plans keeps only over \
                         pointer-carrying names",
                    );
                    debug_assert!(
                        !self.env_regs.iter().any(|e| e.id == reg.id),
                        "an env register rode a root free's keeps — the env's one \
                         owner is its mint"
                    );
                }
            }
        }
        self.emit_frees(&self.shape.root_exit_frees)?;
        let arms = self.shape.arm_root_exit_frees.clone();
        for e in &arms {
            self.emit_arm_free(e)?;
        }
        // The env drain with the shared-cell keeps plan: a box cell one
        // box-is-binding binding shares across several of its envs is
        // Handle-held by each of them, and the runtime's free-time
        // child walk has no refcount — a naive per-env TableFree would
        // free the shared cell once per holder. Envs drain in mint
        // order, so a holder KEEPS every cell a later holder also
        // holds, and the LAST holder's plain TableFree reclaims it:
        // each cell frees exactly once, and the atexit leak check
        // stays balanced.
        let envs = std::mem::take(&mut self.env_regs);
        for (i, env) in envs.iter().enumerate() {
            let mut keeps: Vec<Reg<Ptr>> = Vec::new();
            for (&cell_id, holders) in self.binding_cells.iter() {
                let held_here = holders.iter().any(|e| e.id == env.id);
                let held_later = holders
                    .iter()
                    .any(|e| envs[i + 1..].iter().any(|later| later.id == e.id));
                if held_here && held_later {
                    keeps.push(Reg::new(cell_id));
                }
            }
            if keeps.is_empty() {
                self.emit(Instruction::TableFree { table: *env });
            } else {
                self.emit(Instruction::TableFreeExcept {
                    table: *env,
                    keeps,
                });
            }
        }
        // The unheld cells: boxes minted at declares no env ever took
        // (a capture whose fn literal never became a value on every
        // path that ran the declare). Nothing but the binding itself
        // references them, and no scope pop freed them (the root
        // scope never pops) — the drain frees each once.
        let unheld: Vec<Reg<Ptr>> = self
            .binding_cells
            .iter()
            .filter(|(_, holders)| holders.is_empty())
            .map(|(&id, _)| Reg::<Ptr>::new(id))
            .collect();
        for cell in unheld {
            self.binding_cells.remove(&cell.id);
            self.emit(Instruction::TableFree { table: cell });
        }
        Ok(())
    }

    fn emit_stmt_temp_frees(&mut self, stmt: &Stmt) {
        let Some(sites) = self.shape.stmt_temp_frees.get(&stmt.id()) else {
            return;
        };
        self.emit_site_frees(sites, trace::TRACE_STMT_TEMP_FREE);
    }

    fn emit_cond_temp_frees(&mut self, stmt: &Stmt) {
        let Some(sites) = self.shape.cond_temp_frees.get(&stmt.id()) else {
            return;
        };
        self.emit_site_frees(sites, trace::TRACE_COND_TEMP_FREE);
    }

    fn emit_arm_temp_frees(&mut self, operand: &Expr) {
        let Some(sites) = self.shape.arm_temp_frees.get(&operand.id()) else {
            return;
        };
        self.emit_site_frees(sites, trace::TRACE_ARM_TEMP_FREE);
    }

    fn emit_site_frees(&mut self, sites: &[usize], slot: u8) {
        let mut free_regs: Vec<Reg<Ptr>> = sites
            .iter()
            .filter_map(|site| self.handles.site_regs.get(site).map(|(reg, _)| *reg))
            .collect();
        free_regs.sort_unstable_by_key(|reg| reg.id);
        free_regs.dedup_by_key(|reg| reg.id);
        for reg in free_regs.into_iter().rev() {
            signal!(slot);
            self.emit(Instruction::TableFree { table: reg });
        }
    }

    fn declare_var(
        &mut self,
        name: String,
        reg: AnyReg,
        ty: Ty,
        layout: LayoutVerdict,
    ) -> Result<(), LowerError> {
        let reg = string_repr(&ty, reg);
        // Box-is-binding: a binding the walk says some fn captures
        // gets its ONE cell at declaration — the judge's
        // box-at-declaration model. The cell is the binding's only
        // storage from here on: reads load, writes store, and no mint
        // ever snapshots. A loop body's declare mints per iteration
        // (the TableNew rides the body block) — exactly the judge's
        // semantics. The gate mirrors the mint's boxable faces: the
        // concrete scalars, Any, and Closure. `Tbl` faces alias by
        // pointer (no box element rides them), and the byte-lane tys
        // (Ffi/FfiLib/Async/Unknown) hold handles no env ever boxed —
        // boxing one would store a Ptr value into a Byte lane, the
        // repr law's exact mismatch.
        let boxed = matches!(ty, Ty::Int | Ty::Flt | Ty::Bool | Ty::Str | Ty::Any | Ty::Closure(_))
            && self.captured.contains(&(self.cur_walk_depth(), name.clone()));
        let (bind_reg, cell) = if boxed {
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
            self.table_set(box_reg, TableKey::Int(Reg::new(idx)), reg, &ty)?;
            self.binding_cells.insert(box_id, Vec::new());
            self.scope_cells
                .last_mut()
                .expect("scopes and scope_cells stay index-aligned")
                .insert(box_id);
            (AnyReg::Ptr(box_reg), Some(box_reg))
        } else {
            (reg, None)
        };
        self.push_dbg_bind(&name, bind_reg, None);
        self.scopes.last_mut().unwrap().insert(
            name,
            Local {
                reg: bind_reg,
                cell,
                capture: None,
                ty,
                layout,
            },
        );
        Ok(())
    }

    /// A borrowed cell view: the expansion's face local binding THE
    /// binding's one cell. Unlike `declare_var` this never gates on
    /// the captured union (the view's binding was declared elsewhere)
    /// and never enters `scope_cells` (another scope owns the cell —
    /// a dying scope frees only what it minted).
    fn declare_cell_view(&mut self, name: String, cell: Reg<Ptr>, ty: Ty, layout: LayoutVerdict) {
        self.push_dbg_bind(&name, AnyReg::Ptr(cell), None);
        self.scopes.last_mut().unwrap().insert(
            name,
            Local {
                reg: AnyReg::Ptr(cell),
                cell: Some(cell),
                capture: None,
                ty,
                layout,
            },
        );
    }

    fn update_var(
        &mut self,
        name: &str,
        reg: AnyReg,
        ty: Ty,
        layout: LayoutVerdict,
    ) -> Result<(), LowerError> {
        let reg = string_repr(&ty, reg);
        // The innermost match may be a TABLE capture view of a
        // direct-call expansion — the write lands on the OWNER slot
        // (the view's redirect): the view cannot hold a value, it only
        // re-points reads to the binding's home. Scalar views are
        // cell-backed and route through the cell instead.
        let target = self
            .scopes
            .iter()
            .rposition(|s| s.contains_key(name))
            .ok_or_else(|| LowerError(Diagnostic::lower("Undeclared variable")))?;
        let depth = self.scopes[target]
            .get(name)
            .and_then(|l| l.capture)
            .unwrap_or(target);
        self.update_var_at(name, depth, reg, ty, layout)
    }

    /// The slot-addressed write, mirroring `read_var_at`. A missing
    /// slot is a legitimate fold-back target situation (a closure
    /// escaped the scope that bound the definer's slot), not a
    /// desync — it skips silently.
    fn update_var_at(
        &mut self,
        name: &str,
        depth: usize,
        reg: AnyReg,
        ty: Ty,
        layout: LayoutVerdict,
    ) -> Result<(), LowerError> {
        let Some(scope) = self.scopes.get_mut(depth) else {
            return Ok(());
        };
        let Some(local) = scope.get(name) else {
            return Ok(());
        };
        let cell = local.cell;
        let redirect = local.capture;
        let local_ty = local.ty.clone();
        if let Some(cell) = cell {
            let reg = string_repr(&local_ty, reg);
            let idx = self.next_reg();
            self.emit(Instruction::LoadInt {
                target: Reg::new(idx),
                val: 0,
            });
            self.table_set(cell, TableKey::Int(Reg::new(idx)), reg, &local_ty)?;
            let local = self.scopes[depth].get_mut(name).expect("checked above");
            // The cell's elem was fixed at the declare-time mint — the
            // type is the storage's, not the write's. The layout joins
            // for the reads ahead.
            local.layout = layout;
            return Ok(());
        }
        if let Some(owner) = redirect {
            // A table capture view: the write lands on the owner slot.
            return self.update_var_at(name, owner, reg, ty, layout);
        }
        let reg = string_repr(&ty, reg);
        let local = self.scopes[depth].get_mut(name).expect("checked above");
        local.reg = reg;
        local.ty = ty;
        local.layout = layout;
        self.push_dbg_bind(name, reg, None);
        Ok(())
    }

    /// The value read. A cell-backed binding loads its cell — the
    /// returned register is freshly loaded, never a stale snapshot. A
    /// table capture view resolves at its owner slot.
    fn read_var(&mut self, name: &str) -> Result<Local, LowerError> {
        let mut local = self.peek_var(name)?;
        while let Some(owner) = local.capture {
            local = self.scopes[owner].get(name).cloned().ok_or_else(|| {
                LowerError(Diagnostic::lower("Undeclared variable"))
            })?;
        }
        let reg = local.reg;
        let cell = local.cell;
        let ty = local.ty;
        let layout = local.layout;
        let mut local = Local {
            reg,
            cell: None,
            capture: None,
            ty,
            layout,
        };
        if let Some(cell) = cell {
            local.cell = Some(cell);
            return Ok(self.load_cell_local(&local, cell));
        }
        Ok(local)
    }

    /// The metadata read: no emission, the scope entry as stored. For
    /// a cell-backed binding `reg` is the box pointer — metadata
    /// readers (layouts, keep plans, displaced-table frees) never
    /// take it as a value. A table view redirects to its owner, the
    /// same resolution `read_var` performs.
    fn peek_var(&self, name: &str) -> Result<Local, LowerError> {
        for scope in self.scopes.iter().rev() {
            if let Some(local) = scope.get(name) {
                let mut local = local.clone();
                while let Some(owner) = local.capture {
                    local = self.scopes[owner].get(name).cloned().ok_or_else(|| {
                        LowerError(Diagnostic::lower("Undeclared variable"))
                    })?;
                }
                return Ok(local);
            }
        }
        Err(LowerError(Diagnostic::lower("Undeclared variable")))
    }

    /// One cell load: `cell[0]` into a fresh value register, returned
    /// as the read's Local.
    fn load_cell_local(&mut self, local: &Local, cell: Reg<Ptr>) -> Local {
        let idx = self.next_reg();
        self.emit(Instruction::LoadInt {
            target: Reg::new(idx),
            val: 0,
        });
        let out = self.next_reg();
        let v = self.table_get(out, cell, TableKey::Int(Reg::new(idx)), &local.ty);
        Local {
            reg: v,
            cell: Some(cell),
            capture: None,
            ty: local.ty.clone(),
            layout: local.layout,
        }
    }

    /// A capture's binding, resolved at the walk-scope depth the
    /// checker recorded for it — NOT at the innermost same-named
    /// binding, which a shadow at the mint site would otherwise lend
    /// to the wrong face. The depth is the binding identity shared by
    /// the checker's walk stack and this stack; a miss is a structural
    /// desync between the two, never a program error, so it dies loud.
    fn read_var_at(&self, name: &str, depth: usize) -> Local {
        let scope = self.scopes.get(depth).unwrap_or_else(|| {
            panic!(
                "capture '{name}' resolves to walk-scope {depth}, but \
                 this scope stack holds only {} — the checker recorded \
                 the capture on its walk stack and the lowerer's stack \
                 drifted beneath it: a structural desync, not a \
                 program error",
                self.scopes.len()
            );
        });
        scope.get(name).unwrap_or_else(|| {
            panic!(
                "capture '{name}' has no binding at its recorded \
                 walk-scope {depth} — the checker's chain and this \
                 scope stack disagree on where the name lives: a \
                 structural desync, not a program error"
            );
        }).clone()
    }

    fn has_var(&self, name: &str) -> bool {
        for scope in self.scopes.iter().rev() {
            if scope.contains_key(name) {
                return true;
            }
        }
        false
    }

    fn holds_value(&self, name: &str) -> bool {
        self.peek_var(name)
            .is_ok_and(|local| !matches!(local.ty, Ty::Unknown(_)))
    }

    fn null_moved(&mut self, src: &str) -> Result<(), LowerError> {
        let local = self.peek_var(src)?;
        let null_reg = self.next_reg();
        self.emit(Instruction::LoadNull {
            target: Reg::new(null_reg),
        });
        self.update_var(
            src,
            AnyReg::Ptr(Reg::new(null_reg)),
            local.ty.clone(),
            local.layout,
        )
    }

    fn null_moves_of(&mut self, stmt: &Stmt) -> Result<(), LowerError> {
        if let Some(srcs) = self.shape.move_poisons.get(&stmt.id()) {
            for src in srcs.clone() {
                self.null_moved(&src)?;
            }
        }
        Ok(())
    }
}
