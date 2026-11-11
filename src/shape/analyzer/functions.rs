use super::*;

fn collect_creation_reads_stmt(stmt: &Stmt, out: &mut Vec<String>) {
    match stmt {
        Stmt::LocalDecl { exprs, .. } => {
            for expr in exprs {
                collect_creation_reads_expr(expr, out);
            }
        }
        Stmt::Assignment { expr, .. } => collect_creation_reads_expr(expr, out),
        Stmt::IndexAssign {
            obj, key, value, ..
        } => {
            collect_creation_reads_expr(obj, out);
            collect_creation_reads_expr(key, out);
            collect_creation_reads_expr(value, out);
        }
        Stmt::While {
            condition, body, ..
        } => {
            collect_creation_reads_expr(condition, out);
            for s in body {
                collect_creation_reads_stmt(s, out);
            }
        }
        Stmt::Do { body, .. } => {
            for s in body {
                collect_creation_reads_stmt(s, out);
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            collect_creation_reads_expr(condition, out);
            for s in then_body.iter().chain(else_body.iter()) {
                collect_creation_reads_stmt(s, out);
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(e) = value {
                collect_creation_reads_expr(e, out);
            }
        }
        Stmt::Print { exprs, .. } => {
            for e in exprs {
                collect_creation_reads_expr(e, out);
            }
        }
        Stmt::Expr { expr, .. } => collect_creation_reads_expr(expr, out),
        Stmt::Break { .. } => {}
    }
}

fn collect_creation_reads_expr(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::Identifier { name: n, .. } => out.push(n.clone()),
        Expr::TableCtor { entries, .. } => {
            for (k, v) in entries {
                if let CtorKey::Expr(ke) = k {
                    collect_creation_reads_expr(ke, out);
                }
                collect_creation_reads_expr(v, out);
            }
        }
        Expr::Index { obj, key, .. } => {
            collect_creation_reads_expr(obj, out);
            collect_creation_reads_expr(key, out);
        }
        Expr::Field { obj, .. } => collect_creation_reads_expr(obj, out),
        Expr::BinaryOp { left, right, .. } => {
            collect_creation_reads_expr(left, out);
            collect_creation_reads_expr(right, out);
        }
        Expr::UnaryOp { expr, .. } => collect_creation_reads_expr(expr, out),
        Expr::Call { callee, args, .. } => {
            if !matches!(callee.as_ref(), Expr::Identifier { .. }) {
                collect_creation_reads_expr(callee, out);
            }
            for a in args {
                collect_creation_reads_expr(a, out);
            }
        }
        Expr::Function { .. } => {}
        Expr::Integer { .. }
        | Expr::Float { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Nil { .. }
        | Expr::SysAllocCount { .. }
        | Expr::SysExecCount { .. }
        | Expr::FfiModule { .. }
        | Expr::CoroModule { .. } => {}
    }
}

fn transitive_free_names(stmts: &[Stmt], bound: &mut BTreeSet<String>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for stmt in stmts {
        free_of_stmt(stmt, bound, &mut out);
    }
    out
}

fn free_of_stmt(stmt: &Stmt, bound: &mut BTreeSet<String>, out: &mut BTreeSet<String>) {
    match stmt {
        Stmt::LocalDecl { names, exprs, .. } => {
            for expr in exprs {
                free_of_expr(expr, bound, out);
            }
            for name in names {
                bound.insert(name.clone());
            }
        }
        Stmt::Assignment { name, expr, .. } => {
            free_of_expr(expr, bound, out);
            if !bound.contains(name) {
                out.insert(name.clone());
            }
        }
        Stmt::IndexAssign {
            obj, key, value, ..
        } => {
            free_of_expr(obj, bound, out);
            free_of_expr(key, bound, out);
            free_of_expr(value, bound, out);
        }
        Stmt::While {
            condition, body, ..
        } => {
            free_of_expr(condition, bound, out);
            for s in body {
                free_of_stmt(s, bound, out);
            }
        }
        Stmt::Do { body, .. } => {
            for s in body {
                free_of_stmt(s, bound, out);
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            free_of_expr(condition, bound, out);
            let mut then_bound = bound.clone();
            for s in then_body {
                free_of_stmt(s, &mut then_bound, out);
            }
            let mut else_bound = bound.clone();
            for s in else_body {
                free_of_stmt(s, &mut else_bound, out);
            }
        }
        Stmt::Return { value, .. } => {
            if let Some(e) = value {
                free_of_expr(e, bound, out);
            }
        }
        Stmt::Print { exprs, .. } => {
            for e in exprs {
                free_of_expr(e, bound, out);
            }
        }
        Stmt::Break { .. } => {}
        Stmt::Expr { expr, .. } => free_of_expr(expr, bound, out),
    }
}

fn free_of_expr(expr: &Expr, bound: &BTreeSet<String>, out: &mut BTreeSet<String>) {
    match expr {
        Expr::Identifier { name: n, .. } => {
            if !bound.contains(n) {
                out.insert(n.clone());
            }
        }
        Expr::Integer { .. }
        | Expr::Float { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Nil { .. } => {}
        Expr::TableCtor { entries, .. } => {
            for (k, v) in entries {
                if let CtorKey::Expr(ke) = k {
                    free_of_expr(ke, bound, out);
                }
                free_of_expr(v, bound, out);
            }
        }
        Expr::Index { obj, key, .. } => {
            free_of_expr(obj, bound, out);
            free_of_expr(key, bound, out);
        }
        Expr::Field { obj, .. } => free_of_expr(obj, bound, out),
        Expr::BinaryOp { left, right, .. } => {
            free_of_expr(left, bound, out);
            free_of_expr(right, bound, out);
        }
        Expr::UnaryOp { expr, .. } => free_of_expr(expr, bound, out),
        Expr::Call { callee, args, .. } => {
            if !matches!(callee.as_ref(), Expr::Identifier { .. }) {
                free_of_expr(callee, bound, out);
            }
            for a in args {
                free_of_expr(a, bound, out);
            }
        }
        Expr::Function { params, body, .. } => {
            let mut hole = bound.clone();
            hole.extend(params.iter().cloned());
            for s in body {
                free_of_stmt(s, &mut hole, out);
            }
        }
        Expr::SysAllocCount { .. }
        | Expr::SysExecCount { .. }
        | Expr::FfiModule { .. }
        | Expr::CoroModule { .. } => {}
    }
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
        let depth = self
            .walk
            .fn_scopes
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
                _ => BTreeSet::new(),
            },
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
                _ => {}
            }
            self.ret_base_temp_sites(obj, out);
        }
    }
}
