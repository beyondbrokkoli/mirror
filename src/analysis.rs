use crate::ast::{CtorKey, Expr, NodeId, Stmt};
use std::collections::{BTreeMap, BTreeSet};

pub struct AnalysisContext<'a> {
    pub sites: BTreeMap<NodeId, usize>,
    pub ast: &'a [Stmt],
    pub ctor_lines: Vec<usize>,
    pub fn_touched: BTreeMap<NodeId, BTreeSet<String>>,
}

pub fn build_context<'a>(ast: &'a [Stmt]) -> AnalysisContext<'a> {
    let mut pass = Derivation::new();
    pass.run(ast);
    census_for_desugar(ast);

    AnalysisContext {
        sites: pass.sites,
        ast,
        ctor_lines: pass.ctor_lines,
        fn_touched: pass.fn_touched,
    }
}

struct Derivation {
    sites: BTreeMap<NodeId, usize>,
    ctor_lines: Vec<usize>,
    fn_touched: BTreeMap<NodeId, BTreeSet<String>>,
    seen: BTreeSet<NodeId>,
    scopes: Vec<BTreeSet<String>>,
}

struct FnScan {
    entry_depth: usize,
    touched: BTreeSet<String>,
}

impl Derivation {
    fn new() -> Self {
        Derivation {
            sites: BTreeMap::new(),
            ctor_lines: Vec::new(),
            fn_touched: BTreeMap::new(),
            seen: BTreeSet::new(),
            scopes: vec![BTreeSet::new()],
        }
    }

    fn run(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            self.visit_stmt(s, None);
        }
    }

    fn visit_stmts(&mut self, stmts: &[Stmt], mut current_fn: Option<&mut FnScan>) {
        for s in stmts {
            self.visit_stmt(s, current_fn.as_deref_mut());
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt, mut current_fn: Option<&mut FnScan>) {
        assert!(
            self.seen.insert(stmt.id()),
            "identity census (H6): stmt id {} appears twice — a cloned subtree \
             reached the tree; one construction mints one node, a re-materialized \
             subtree must be constructed, never cloned",
            stmt.id().0
        );
        match stmt {
            Stmt::LocalDecl { names, exprs, .. } => {
                for e in exprs {
                    self.visit_expr(e, current_fn.as_deref_mut());
                }
                self.scopes
                    .last_mut()
                    .unwrap()
                    .extend(names.iter().cloned());
            }
            Stmt::Assignment { name, expr, .. } => {
                record_touched(current_fn.as_deref_mut(), &self.scopes, name);
                self.visit_expr(expr, current_fn);
            }
            Stmt::IndexAssign {
                obj, key, value, ..
            } => {
                if let Some(base) = base_identifier(obj) {
                    record_touched(current_fn.as_deref_mut(), &self.scopes, base);
                }
                self.visit_expr(obj, current_fn.as_deref_mut());
                self.visit_expr(key, current_fn.as_deref_mut());
                self.visit_expr(value, current_fn);
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.visit_expr(condition, current_fn.as_deref_mut());
                self.scopes.push(Default::default());
                self.visit_stmts(body, current_fn);
                self.scopes.pop();
            }
            Stmt::Do { body, .. } => {
                self.scopes.push(Default::default());
                self.visit_stmts(body, current_fn);
                self.scopes.pop();
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.visit_expr(condition, current_fn.as_deref_mut());
                self.scopes.push(Default::default());
                self.visit_stmts(then_body, current_fn.as_deref_mut());
                self.scopes.pop();
                self.scopes.push(Default::default());
                self.visit_stmts(else_body, current_fn);
                self.scopes.pop();
            }
            Stmt::Print { exprs, .. } => {
                for e in exprs {
                    self.visit_expr(e, current_fn.as_deref_mut());
                }
            }
            Stmt::Expr { expr, .. } => self.visit_expr(expr, current_fn),
            Stmt::Return { value, .. } => {
                if let Some(e) = value {
                    self.visit_expr(e, current_fn);
                }
            }
            Stmt::Break { .. } => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr, mut current_fn: Option<&mut FnScan>) {
        assert!(
            self.seen.insert(expr.id()),
            "identity census (H6): expr id {} appears twice — a cloned subtree \
             reached the tree; one construction mints one node, a re-materialized \
             subtree must be constructed, never cloned",
            expr.id().0
        );
        match expr {
            Expr::TableCtor { entries, line, .. } => {
                self.sites.insert(expr.id(), self.sites.len());
                self.ctor_lines.push(*line as usize);
                for (k, v) in entries {
                    if let CtorKey::Expr(ke) = k {
                        self.visit_expr(ke, current_fn.as_deref_mut());
                    }
                    self.visit_expr(v, current_fn.as_deref_mut());
                }
            }
            Expr::Field { obj, .. } => {
                self.visit_expr(obj, current_fn);
            }
            Expr::Index { obj, key, .. } => {
                self.visit_expr(obj, current_fn.as_deref_mut());
                self.visit_expr(key, current_fn);
            }
            Expr::BinaryOp { left, right, .. } => {
                self.visit_expr(left, current_fn.as_deref_mut());
                self.visit_expr(right, current_fn);
            }
            Expr::UnaryOp { expr, .. } => self.visit_expr(expr, current_fn),
            Expr::Call { callee, args, .. } => {
                self.visit_expr(callee, current_fn.as_deref_mut());
                for a in args {
                    self.visit_expr(a, current_fn.as_deref_mut());
                }
            }
            Expr::Function { params, body, .. } => {
                let entry_depth = self.scopes.len();
                self.scopes.push(params.iter().cloned().collect());
                let mut inner = FnScan {
                    entry_depth,
                    touched: BTreeSet::new(),
                };
                self.visit_stmts(body, Some(&mut inner));
                self.scopes.pop();
                if let Some(outer) = current_fn {
                    outer.touched.extend(inner.touched.iter().cloned());
                }
                self.fn_touched.insert(expr.id(), inner.touched);
            }
            _ => {}
        }
    }
}

fn record_touched(current_fn: Option<&mut FnScan>, scopes: &[BTreeSet<String>], name: &str) {
    if let Some(scan) = current_fn
        && !scopes[scan.entry_depth..].iter().any(|s| s.contains(name))
    {
        scan.touched.insert(name.to_string());
    }
}

fn base_identifier(expr: &Expr) -> Option<&String> {
    match expr {
        Expr::Identifier { name, .. } => Some(name),
        Expr::Index { obj, .. } => base_identifier(obj),
        _ => None,
    }
}

fn census_for_desugar(stmts: &[Stmt]) {
    for s in stmts {
        if let Stmt::Do { body, .. } = s
            && matches!(
                body.get(1),
                Some(Stmt::LocalDecl { names, .. })
                    if names.len() == 1 && names[0] == "for$limit"
            )
        {
            let header = s.loc().line;
            let check = |what: &str, node: &Stmt| {
                let l = node.loc().line;
                if l != header {
                    panic!(
                        "for-desugar attribution drift: the {what} reports line {l} but the for \
                         header reports line {header} — every desugared statement must carry the \
                         header's stamp (R7's law, H5's census)"
                    );
                }
            };
            check("counter decl", &body[0]);
            check("for$limit latch decl", &body[1]);
            let Stmt::While { body: wbody, .. } = &body[2] else {
                panic!(
                    "for-desugar census: the block's third statement is not the While — the \
                     desugar shape changed; update the census with it (H5)"
                );
            };
            check("While", &body[2]);
            match (wbody.first(), wbody.last()) {
                (Some(inner), Some(inc)) if wbody.len() == 2 => {
                    check("body's inner Do", inner);
                    check("synthetic increment", inc);
                }
                _ => panic!(
                    "for-desugar census: the While body is not [inner Do, increment] — the \
                     desugar shape changed; update the census with it (H5)"
                ),
            }
        }
        if let Stmt::Do { body, .. } = s
            && matches!(
                body.first(),
                Some(Stmt::LocalDecl { names, .. })
                    if !names.is_empty() && names[0] == "tuple$t0"
            )
        {
            let header = s.loc().line;
            let check = |what: &str, node: &Stmt| {
                let l = node.loc().line;
                if l != header {
                    panic!(
                        "tuple-assign attribution drift: the {what} reports line {l} but the \
                         tuple header reports line {header} — every desugared statement must \
                         carry the header's stamp (R7's law, H5's census)"
                    );
                }
            };
            let Stmt::LocalDecl { names, .. } = &body[0] else {
                unreachable!("the fingerprint arm matched a LocalDecl")
            };
            for (i, name) in names.iter().enumerate() {
                if name != &format!("tuple$t{i}") {
                    panic!(
                        "tuple-assign census: temp {i} is named '{name}', not 'tuple$t{i}' — \
                         the desugar shape changed; update the census with it (H8)"
                    );
                }
            }
            check("temp decl", &body[0]);
            if body.len() != names.len() + 1 {
                panic!(
                    "tuple-assign census: {} temps but {} statements in the block — the desugar \
                     shape changed; update the census with it (H8)",
                    names.len(),
                    body.len() - 1
                );
            }
            for (i, stmt) in body[1..].iter().enumerate() {
                let Stmt::IndexAssign { value, .. } = stmt else {
                    panic!(
                        "tuple-assign census: the block's statement {} is not the cell store — \
                         the desugar shape changed; update the census with it (H8)",
                        i + 1
                    );
                };
                if !matches!(value, Expr::Identifier { name: v, .. } if v == &format!("tuple$t{i}"))
                {
                    panic!(
                        "tuple-assign census: store {i} does not read temp {i} — the desugar \
                         shape changed; update the census with it (H8)"
                    );
                }
                check(&format!("cell store {i}"), stmt);
            }
        }
        match s {
            Stmt::While { body, .. } | Stmt::Do { body, .. } => census_for_desugar(body),
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                census_for_desugar(then_body);
                census_for_desugar(else_body);
            }
            _ => {}
        }
        census_for_desugar_in_exprs(s);
    }
}

fn census_for_desugar_in_exprs(stmt: &Stmt) {
    let walk = census_expr;
    match stmt {
        Stmt::LocalDecl { exprs, .. } | Stmt::Print { exprs, .. } => {
            for e in exprs {
                walk(e);
            }
        }
        Stmt::Assignment { expr, .. } | Stmt::Expr { expr, .. } => walk(expr),
        Stmt::IndexAssign {
            obj, key, value, ..
        } => {
            walk(obj);
            walk(key);
            walk(value);
        }
        Stmt::While { condition, .. } => walk(condition),
        Stmt::If { condition, .. } => walk(condition),
        Stmt::Return { value, .. } => {
            if let Some(e) = value {
                walk(e);
            }
        }
        Stmt::Break { .. } | Stmt::Do { .. } => {}
    }
}

fn census_expr(e: &Expr) {
    match e {
        Expr::Function { body, .. } => census_for_desugar(body),
        Expr::TableCtor { entries, .. } => {
            for (k, v) in entries {
                if let CtorKey::Expr(ke) = k {
                    census_expr(ke);
                }
                census_expr(v);
            }
        }
        Expr::Field { obj, .. } => census_expr(obj),
        Expr::Index { obj, key, .. } => {
            census_expr(obj);
            census_expr(key);
        }
        Expr::BinaryOp { left, right, .. } => {
            census_expr(left);
            census_expr(right);
        }
        Expr::UnaryOp { expr, .. } => census_expr(expr),
        Expr::Call { callee, args, .. } => {
            census_expr(callee);
            for a in args {
                census_expr(a);
            }
        }
        _ => {}
    }
}
