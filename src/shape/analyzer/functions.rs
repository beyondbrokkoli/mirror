use super::*;
use crate::ast::Loc;

/// Names read by a creation site — rides the traversal engine. Two laws
/// carried over from the hand walker: a function literal's body is not
/// descended (its reads belong to the closure's own accounting), and a
/// direct-identifier callee is a call, not a read.
struct CreationReads {
    out: Vec<String>,
}

impl crate::visit::Visitor for CreationReads {
    type Break = std::convert::Infallible;

    fn visit_identifier(&mut self, name: &str, _id: NodeId) -> Result<(), Self::Break> {
        self.out.push(name.to_string());
        Ok(())
    }

    fn visit_call(&mut self, callee: &Expr, args: &[Expr], _id: NodeId) -> Result<(), Self::Break> {
        if !matches!(callee, Expr::Identifier { .. }) {
            crate::visit::walk_expr(self, callee)?;
        }
        for a in args {
            crate::visit::walk_expr(self, a)?;
        }
        Ok(())
    }

    fn visit_function(
        &mut self,
        _params: &[String],
        _body: &[Stmt],
        _id: NodeId,
    ) -> Result<(), Self::Break> {
        Ok(())
    }
}

fn collect_creation_reads_stmt(stmt: &Stmt, out: &mut Vec<String>) {
    let mut v = CreationReads {
        out: std::mem::take(out),
    };
    let _ = crate::visit::walk_stmt(&mut v, stmt);
    *out = v.out;
}

/// Free names of a body, transitively — rides the traversal engine with
/// the bound-set as visitor state. If branches walk on independent clones
/// (a binding in one arm must not satisfy the other); a function literal
/// binds its params for its body only.
struct FreeNames {
    bound: BTreeSet<String>,
    out: BTreeSet<String>,
}

impl crate::visit::Visitor for FreeNames {
    type Break = std::convert::Infallible;

    fn visit_identifier(&mut self, name: &str, _id: NodeId) -> Result<(), Self::Break> {
        if !self.bound.contains(name) {
            self.out.insert(name.to_string());
        }
        Ok(())
    }

    fn visit_local_decl(
        &mut self,
        names: &[String],
        exprs: &[Expr],
        _loc: Loc,
        _id: NodeId,
    ) -> Result<(), Self::Break> {
        for e in exprs {
            crate::visit::walk_expr(self, e)?;
        }
        for name in names {
            self.bound.insert(name.clone());
        }
        Ok(())
    }

    fn visit_assignment(
        &mut self,
        name: &str,
        expr: &Expr,
        _loc: Loc,
        _id: NodeId,
    ) -> Result<(), Self::Break> {
        crate::visit::walk_expr(self, expr)?;
        if !self.bound.contains(name) {
            self.out.insert(name.to_string());
        }
        Ok(())
    }

    fn visit_if(
        &mut self,
        condition: &Expr,
        then_body: &[Stmt],
        else_body: &[Stmt],
        _loc: Loc,
        _id: NodeId,
    ) -> Result<(), Self::Break> {
        crate::visit::walk_expr(self, condition)?;
        let entry = self.bound.clone();
        crate::visit::walk_stmts(self, then_body)?;
        self.bound = entry.clone();
        crate::visit::walk_stmts(self, else_body)?;
        self.bound = entry;
        Ok(())
    }

    fn visit_call(&mut self, callee: &Expr, args: &[Expr], _id: NodeId) -> Result<(), Self::Break> {
        if !matches!(callee, Expr::Identifier { .. }) {
            crate::visit::walk_expr(self, callee)?;
        }
        for a in args {
            crate::visit::walk_expr(self, a)?;
        }
        Ok(())
    }

    fn visit_function(
        &mut self,
        params: &[String],
        body: &[Stmt],
        _id: NodeId,
    ) -> Result<(), Self::Break> {
        let saved = self.bound.clone();
        self.bound.extend(params.iter().cloned());
        let _ = crate::visit::walk_stmts(self, body);
        self.bound = saved;
        Ok(())
    }
}

fn transitive_free_names(stmts: &[Stmt], bound: &mut BTreeSet<String>) -> BTreeSet<String> {
    let mut v = FreeNames {
        bound: std::mem::take(bound),
        out: BTreeSet::new(),
    };
    let _ = crate::visit::walk_stmts(&mut v, stmts);
    *bound = v.bound;
    v.out
}

impl Analyzer {
    pub(super) fn resolve_fn(&self, name: &str) -> Option<NodeId> {
        for scope in self.walk.fn_scopes.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return *entry;
            }
        }
        None
    }

    pub(super) fn declare_fn_binding(&mut self, name: &str, ptr: Option<NodeId>) {
        self.walk
            .fn_scopes
            .last_mut()
            .unwrap()
            .insert(name.to_string(), ptr);
    }

    pub(super) fn assign_fn_binding(&mut self, name: &str, ptr: Option<NodeId>) {
        // A fn entry rides its name's binding depth, not wherever a
        // stale entry happens to sit: scanning the binding scopes (which
        // every binder — decl, param, assignment — writes at its home
        // depth) so a nested assignment may not orphan the fn-ness in
        // an inner map that dies with its scope, or park it at a stale
        // rider depth the master stack never sanctioned. `scopes` and
        // `fn_scopes` are index-aligned (enter/exit, resets); the
        // fallback covers the name bound nowhere, which the
        // undeclared-assignment path binds innermost right after this
        // write.
        let depth = self
            .walk
            .scopes
            .iter()
            .rposition(|s| s.contains_key(name))
            .unwrap_or(self.walk.fn_scopes.len() - 1);
        self.walk.fn_scopes[depth].insert(name.to_string(), ptr);
    }

    pub(super) fn walk_fn_def(
        &mut self,
        rec: Gate,
        fn_ptr: NodeId,
        params: &[String],
        body: &[Stmt],
        key: (NodeId, u8),
    ) -> Result<(), ShapeError> {
        self.fn_scope_keys.insert(fn_ptr, key);
        let mut chain: BTreeMap<String, Option<NodeId>> = BTreeMap::new();
        for scope in self.walk.fn_scopes.iter().skip(1) {
            for (n, p) in scope {
                chain.insert(n.clone(), *p);
            }
        }
        self.fn_def_sites.insert(fn_ptr, chain.clone());
        let mut bound = BTreeSet::new();
        bound.extend(params.iter().cloned());
        let free = transitive_free_names(body, &mut bound);
        let captures: BTreeSet<String> = free
            .into_iter()
            .filter(|n| chain.get(n) == Some(&None))
            .collect();
        self.fn_captures.insert(fn_ptr, captures);
        self.lattice.unknowns.ensure_fn_ret(fn_ptr);
        collect_body_sites(body, &self.lattice.sites, &mut self.fn_body_sites);
        self.fn_params.insert(fn_ptr, params.to_vec());
        let stored: BTreeSet<String> = collect_stored_bases(body)
            .into_iter()
            .filter(|n| params.contains(n))
            .collect();
        self.fn_store_params.insert(fn_ptr, stored);
        self.fn_rets.push(None);
        self.fn_edge_frames.push(Vec::new());
        self.fn_ret_reaches.push(BTreeSet::new());
        self.fn_walk.push((fn_ptr, params.to_vec()));
        self.fn_scope_entries.push(self.walk.scopes.len());
        let mut result = Ok(());
        self.with_scope(|an| {
            let scope = an.walk.scopes.last_mut().unwrap();
            for p in params {
                // A parameter is a fresh binding whose value is the
                // caller's argument: record it as the unknowable shape
                // so an outer same-named literal cannot lend its
                // nonneg/lower-bound facts to a loop over the
                // parameter — the entry facts would describe the
                // caller's argument, not the literal (the F3 probe).
                an.holds.num_assigns.insert(p.clone(), vec![NumExpr::Other]);
                scope.insert(
                    p.clone(),
                    TableShape {
                        ty: Pending,
                        layout: LayoutVerdict::default(),
                        aliases: BTreeSet::new(),
                        lineage: BTreeMap::new(),
                    },
                );
            }
            let fns = an.walk.fn_scopes.last_mut().unwrap();
            for p in params {
                fns.insert(p.clone(), None);
            }
            an.walk_stmts(rec, body);
            if let Err(err) = an.decide_scope_exit(rec, key) {
                result = Err(err);
            }
        });
        self.fn_scope_entries.pop();
        self.fn_walk.pop();
        let joined = self.fn_rets.pop().unwrap_or(None);
        let edges = self.fn_edge_frames.pop().unwrap_or_default();
        let joined_reach = self.fn_ret_reaches.pop().unwrap_or_default();
        if joined.is_some() {
            self.fn_ret_reach.insert(fn_ptr, joined_reach);
        }
        if !edges.is_empty() {
            self.fn_ret_edge_shapes.insert(fn_ptr, edges);
        }
        if let Some(shape) = joined {
            self.fn_ret_shapes.insert(fn_ptr, shape);
        }
        result
    }

    pub(super) fn sever_creation_captures(&mut self, rec: Gate, stmt: &Stmt) {
        let mut readers = Vec::new();
        collect_creation_reads_stmt(stmt, &mut readers);
        if readers.is_empty() {
            return;
        }
        let mut severed: Vec<(usize, String)> = Vec::new();
        for name in readers {
            let Some(ptr) = self.resolve_fn(&name) else {
                continue;
            };
            let Some(caps) = self.fn_captures.get(&ptr).cloned() else {
                continue;
            };
            for cap in caps {
                if let Ok((d, bind)) = self.resolve(rec, &cap)
                    && matches!(bind.ty, Tbl(_))
                    && !severed.iter().any(|(_, n)| n == &cap)
                {
                    severed.push((d, cap));
                }
            }
        }
        if severed.is_empty() {
            return;
        }
        for (d, cap) in &severed {
            self.poison_moved(*d, cap);
        }
        if rec.on() {
            let entry = self.own.move_poisons.entry(stmt.id()).or_default();
            for (_, cap) in &severed {
                entry.insert(cap.clone());
            }
        }
    }

    pub(super) fn return_reaches_param(&self, expr: &Expr, params: &[String]) -> BTreeSet<usize> {
        match expr {
            Expr::Identifier { name: n, .. } => {
                params.iter().position(|p| p == n).into_iter().collect()
            }
            Expr::Call { callee, args, .. } => match callee.as_ref() {
                Expr::Identifier { name, .. } => {
                    let Some(fn_ptr) = self.resolve_fn(name) else {
                        return BTreeSet::new();
                    };
                    let Some(ret) = self.fn_ret_params.get(&fn_ptr) else {
                        return BTreeSet::new();
                    };
                    let mut out = BTreeSet::new();
                    for &j in ret {
                        if let Some(arg) = args.get(j) {
                            out.extend(self.return_reaches_param(arg, params));
                        }
                    }
                    out
                }
                // C: only a direct-name callee resolves — every other
                // callee shape has no param reach to trace
                _ => BTreeSet::new(),
            },
            // C: only identifiers and calls carry a param's value out
            _ => BTreeSet::new(),
        }
    }

    pub(super) fn record_moved_param(&mut self, name: &str) {
        if let Some((fn_ptr, params)) = self.fn_walk.last()
            && params.iter().any(|p| p == name)
        {
            self.fn_moved_params
                .entry(*fn_ptr)
                .or_default()
                .insert(name.to_string());
        }
    }

    pub(super) fn check_boundary_pin(
        &self,
        bind: &TableShape,
        src: &str,
    ) -> Result<(), ShapeError> {
        if bind.aliases.contains(&BOUNDARY_ROOT) {
            return Err(ShapeError(Diagnostic::lifetime(format!(
                "the boundary table '{src}' is pinned — the host owns \
                 its header; read its cells ('{src}[i]') instead of moving the table"
            ))));
        }
        Ok(())
    }

    pub(super) fn ret_reach_sites(&self, e: &Expr, out: &mut BTreeSet<usize>) {
        match e {
            Expr::Identifier { name: n, .. } => {
                if let Ok((_, bind)) = self.resolve(None, n) {
                    out.extend(bind.aliases.iter().copied().filter(|s| !is_root(s)));
                    out.extend(bind.lineage.keys().copied().filter(|s| !is_root(s)));
                }
            }
            Expr::Index { obj, .. } => self.ret_reach_sites(obj, out),
            Expr::Call { callee, args, .. } => {
                if let Expr::Identifier { name, .. } = callee.as_ref()
                    && let Some(fn_ptr) = self.resolve_fn(name)
                {
                    if let Some(reach) = self.fn_ret_reach.get(&fn_ptr) {
                        out.extend(reach.iter().copied());
                    }
                    if let Some(rets) = self.fn_ret_params.get(&fn_ptr) {
                        for &i in rets {
                            if let Some(a) = args.get(i) {
                                self.ret_reach_sites(a, out);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn ret_base_temp_sites(&self, e: &Expr, out: &mut Vec<usize>) {
        if let Expr::Index { obj, .. } = e {
            match obj.as_ref() {
                Expr::TableCtor { .. } => {
                    if let Some(&site) = self.lattice.sites.get(&obj.as_ref().id()) {
                        out.push(site);
                    }
                }
                Expr::Call { callee, .. } => {
                    if let Expr::Identifier { name, .. } = callee.as_ref()
                        && let Some(fn_ptr) = self.resolve_fn(name)
                        && let Some(shape) = self.fn_ret_shapes.get(&fn_ptr)
                    {
                        out.extend(
                            shape
                                .aliases
                                .iter()
                                .copied()
                                .filter(|s| !is_root(s) && !is_ghost(s)),
                        );
                    }
                }
                // C: any other base is neither a fresh ctor nor a fn return
                _ => {}
            }
            self.ret_base_temp_sites(obj, out);
        }
    }
}
