
// The touched-name collectors ride the traversal engine. One law carries
// over from the hand walkers: a function *literal's* body is not descended
// (its names are reached through the call cross-edge below), so
// visit_function skips — everything else is the engine's default.

use crate::visit::{self, Visitor};
use std::convert::Infallible;

struct FnTouch<'a> {
    shape: &'a ShapeFacts,
    out: &'a mut BTreeSet<String>,
    seen: &'a mut BTreeSet<NodeId>,
    /// read `move_poisons` at every statement (find_moved_names)
    poisons: bool,
    /// record plain-assignment names (find_mutated_vars)
    mutated: bool,
}

impl Visitor for FnTouch<'_> {
    type Break = Infallible;

    fn enter_stmt(&mut self, stmt: &Stmt) -> Result<(), Self::Break> {
        if self.poisons
            && let Some(names) = self.shape.move_poisons.get(&stmt.id())
        {
            self.out.extend(names.iter().cloned());
        }
        Ok(())
    }

    fn visit_assignment(
        &mut self,
        name: &str,
        expr: &Expr,
        _loc: crate::ast::Loc,
        _id: NodeId,
    ) -> Result<(), Self::Break> {
        if self.mutated {
            self.out.insert(name.to_string());
        }
        visit::walk_expr(self, expr)
    }

    fn visit_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        id: NodeId,
    ) -> Result<(), Self::Break> {
        if let Some(fns) = self.shape.call_fns.get(&id) {
            for &fn_ptr in fns {
                if let Some(touched) = self.shape.fn_touched.get(&fn_ptr) {
                    self.out.extend(touched.iter().cloned());
                }
                if self.seen.insert(fn_ptr)
                    && let Some(def) = self.shape.fn_defs.get(&fn_ptr)
                {
                    // cross-edge into the callee's body: same collector,
                    // fresh walk — poisons and mutation recording belong
                    // to the statement lane, not the call edge
                    let shape = self.shape;
                    let out = &mut *self.out;
                    let seen = &mut *self.seen;
                    let mut edge = FnTouch {
                        shape,
                        out,
                        seen,
                        poisons: false,
                        mutated: false,
                    };
                    let _ = visit::walk_stmts(&mut edge, def.body());
                }
            }
        }
        visit::walk_expr(self, callee)?;
        for a in args {
            visit::walk_expr(self, a)?;
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

fn const_of(expr: &Expr) -> Option<i64> {
    // C: single-node classifier — only an Integer is a constant.
    match expr {
        Expr::Integer { value: v, .. } => Some(*v),
        Expr::Float { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Nil { .. }
        | Expr::TableCtor { .. }
        | Expr::Identifier { .. }
        | Expr::Index { .. }
        | Expr::BinaryOp { .. }
        | Expr::UnaryOp { .. }
        | Expr::SysAllocCount { .. }
        | Expr::SysExecCount { .. }
        | Expr::FfiModule { .. }
        | Expr::AsyncModule { .. }
        | Expr::Field { .. }
        | Expr::Function { .. }
        | Expr::Call { .. } => None,
    }
}

fn as_ident(expr: &Expr) -> Option<&str> {
    // C: single-node classifier — only an Identifier names a variable.
    match expr {
        Expr::Identifier { name, .. } => Some(name),
        Expr::Integer { .. }
        | Expr::Float { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Nil { .. }
        | Expr::TableCtor { .. }
        | Expr::Index { .. }
        | Expr::BinaryOp { .. }
        | Expr::UnaryOp { .. }
        | Expr::SysAllocCount { .. }
        | Expr::SysExecCount { .. }
        | Expr::FfiModule { .. }
        | Expr::AsyncModule { .. }
        | Expr::Field { .. }
        | Expr::Function { .. }
        | Expr::Call { .. } => None,
    }
}

fn find_moved_names(stmts: &[Stmt], shape: &ShapeFacts) -> BTreeSet<String> {
    let mut moved = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut v = FnTouch {
        shape,
        out: &mut moved,
        seen: &mut seen,
        poisons: true,
        mutated: false,
    };
    let _ = visit::walk_stmts(&mut v, stmts);
    moved
}

fn find_mutated_vars(stmts: &[Stmt], shape: &ShapeFacts) -> BTreeSet<String> {
    let mut mutated = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut v = FnTouch {
        shape,
        out: &mut mutated,
        seen: &mut seen,
        poisons: false,
        mutated: true,
    };
    let _ = visit::walk_stmts(&mut v, stmts);
    mutated
}
