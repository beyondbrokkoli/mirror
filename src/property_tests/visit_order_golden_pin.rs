// The children-order law's own lock: a recording visitor pins the engine's
// traversal order for every Stmt/Expr variant. If a children-set or its
// order changes, this golden changes with it — that is the point.

use crate::ast::{CtorKey, Loc};
use crate::visit::{self as ve, Visitor};
use std::convert::Infallible;

struct VisitRecorder {
    log: Vec<&'static str>,
}

impl Visitor for VisitRecorder {
    type Break = Infallible;

    fn visit_do(&mut self, b: &[Stmt], _: Loc, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("do");
        ve::walk_stmts(self, b)
    }
    fn visit_break(&mut self, _: Loc, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("break");
        Ok(())
    }
    fn visit_assignment(
        &mut self,
        _n: &str,
        e: &Expr,
        _: Loc,
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("assignment");
        ve::walk_expr(self, e)
    }
    fn visit_index_assign(
        &mut self,
        o: &Expr,
        k: &Expr,
        v: &Expr,
        _: Loc,
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("index_assign");
        ve::walk_expr(self, o)?;
        ve::walk_expr(self, k)?;
        ve::walk_expr(self, v)
    }
    fn visit_while(
        &mut self,
        c: &Expr,
        b: &[Stmt],
        _: Loc,
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("while");
        ve::walk_expr(self, c)?;
        ve::walk_stmts(self, b)
    }
    fn visit_if(
        &mut self,
        c: &Expr,
        t: &[Stmt],
        e: &[Stmt],
        _: Loc,
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("if");
        ve::walk_expr(self, c)?;
        ve::walk_stmts(self, t)?;
        ve::walk_stmts(self, e)
    }
    fn visit_print(&mut self, es: &[Expr], _: Loc, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("print");
        for e in es {
            ve::walk_expr(self, e)?;
        }
        Ok(())
    }
    fn visit_expr_stmt(&mut self, e: &Expr, _: Loc, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("expr_stmt");
        ve::walk_expr(self, e)
    }
    fn visit_return(
        &mut self,
        v: Option<&Expr>,
        _: Loc,
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("return");
        if let Some(e) = v {
            ve::walk_expr(self, e)?;
        }
        Ok(())
    }
    fn visit_local_decl(
        &mut self,
        _names: &[String],
        exprs: &[Expr],
        _: Loc,
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("local_decl");
        for e in exprs {
            ve::walk_expr(self, e)?;
        }
        Ok(())
    }
    fn visit_integer(&mut self, _: i64, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("integer");
        Ok(())
    }
    fn visit_float(&mut self, _: f64, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("float");
        Ok(())
    }
    fn visit_boolean(&mut self, _: bool, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("boolean");
        Ok(())
    }
    fn visit_string(&mut self, _: &str, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("string");
        Ok(())
    }
    fn visit_nil(&mut self, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("nil");
        Ok(())
    }
    fn visit_table_ctor(
        &mut self,
        entries: &[(CtorKey, Expr)],
        _: u32,
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("table_ctor");
        for (k, v) in entries {
            if let CtorKey::Expr(ke) = k {
                ve::walk_expr(self, ke)?;
            }
            ve::walk_expr(self, v)?;
        }
        Ok(())
    }
    fn visit_identifier(&mut self, _: &str, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("identifier");
        Ok(())
    }
    fn visit_index(&mut self, o: &Expr, k: &Expr, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("index");
        ve::walk_expr(self, o)?;
        ve::walk_expr(self, k)
    }
    fn visit_binary_op(
        &mut self,
        _: &BinOp,
        l: &Expr,
        r: &Expr,
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("binary_op");
        ve::walk_expr(self, l)?;
        ve::walk_expr(self, r)
    }
    fn visit_unary_op(
        &mut self,
        _: &crate::ast::UnOp,
        e: &Expr,
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("unary_op");
        ve::walk_expr(self, e)
    }
    fn visit_sys_alloc_count(&mut self, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("sys_alloc_count");
        Ok(())
    }
    fn visit_sys_exec_count(&mut self, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("sys_exec_count");
        Ok(())
    }
    fn visit_ffi_module(&mut self, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("ffi_module");
        Ok(())
    }
    fn visit_async_module(&mut self, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("async_module");
        Ok(())
    }
    fn visit_field(&mut self, o: &Expr, _: &str, _: NodeId) -> Result<(), Self::Break> {
        self.log.push("field");
        ve::walk_expr(self, o)
    }
    fn visit_function(
        &mut self,
        _params: &[String],
        b: &[Stmt],
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("function");
        ve::walk_stmts(self, b)
    }
    fn visit_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        _: NodeId,
    ) -> Result<(), Self::Break> {
        self.log.push("call");
        ve::walk_expr(self, callee)?;
        for a in args {
            ve::walk_expr(self, a)?;
        }
        Ok(())
    }
}

#[test]
fn visit_order_golden_pin() {
    let at = Loc { line: 1, col: 1 };
    let mint = NodeId::mint;
    let ident = |n: &str| Expr::Identifier {
        name: n.to_string(),
        id: mint(),
    };
    let int = |v: i64| Expr::Integer { value: v, id: mint() };

    // One AST that exercises every variant at least once.
    let ast = vec![
        Stmt::LocalDecl {
            names: vec!["t".into()],
            exprs: vec![Expr::TableCtor {
                entries: vec![
                    (CtorKey::Const(1), int(10)),
                    (CtorKey::Expr(ident("k")), int(20)),
                    (CtorKey::Str("s".into()), int(30)),
                ],
                line: 1,
                id: mint(),
            }],
            loc: at,
            id: mint(),
        },
        Stmt::Assignment {
            name: "x".into(),
            expr: Expr::BinaryOp {
                op: BinOp::Add,
                left: Box::new(int(1)),
                right: Box::new(Expr::UnaryOp {
                    op: crate::ast::UnOp::Neg,
                    expr: Box::new(Expr::Float {
                        value: 2.0,
                        id: mint(),
                    }),
                    id: mint(),
                }),
                id: mint(),
            },
            loc: at,
            id: mint(),
        },
        Stmt::IndexAssign {
            obj: Expr::Index {
                obj: Box::new(ident("t")),
                key: Box::new(int(1)),
                id: mint(),
            },
            key: Expr::Boolean {
                value: true,
                id: mint(),
            },
            value: Expr::Nil { id: mint() },
            loc: at,
            id: mint(),
        },
        Stmt::While {
            condition: Expr::Boolean {
                value: false,
                id: mint(),
            },
            body: vec![Stmt::Break {
                loc: at,
                id: mint(),
            }],
            loc: at,
            id: mint(),
        },
        Stmt::Do {
            body: vec![Stmt::Expr {
                expr: Expr::String {
                    value: "v".into(),
                    id: mint(),
                },
                loc: at,
                id: mint(),
            }],
            loc: at,
            id: mint(),
        },
        Stmt::If {
            condition: Expr::SysAllocCount { id: mint() },
            then_body: vec![Stmt::Print {
                exprs: vec![
                    Expr::SysExecCount { id: mint() },
                    Expr::FfiModule { id: mint() },
                    Expr::AsyncModule { id: mint() },
                ],
                loc: at,
                id: mint(),
            }],
            else_body: vec![],
            loc: at,
            id: mint(),
        },
        Stmt::Expr {
            expr: Expr::Call {
                callee: Box::new(Expr::Field {
                    obj: Box::new(ident("ffi")),
                    name: "open".into(),
                    id: mint(),
                }),
                args: vec![ident("k")],
                id: mint(),
            },
            loc: at,
            id: mint(),
        },
        Stmt::Return {
            value: Some(Expr::Function {
                params: vec!["a".into()],
                body: vec![Stmt::Return {
                    value: Some(ident("a")),
                    loc: at,
                    id: mint(),
                }],
                id: mint(),
            }),
            loc: at,
            id: mint(),
        },
    ];

    let mut rec = VisitRecorder { log: Vec::new() };
    let _ = ve::walk_stmts(&mut rec, &ast);
    assert_eq!(
        rec.log,
        vec![
            // local decl with the table ctor: keys before values, entry order
            "local_decl",
            "table_ctor",
            "integer",    // Const(1) -> 10
            "identifier", // Expr key k
            "integer",    //   -> 20
            "integer",    // Str("s") -> 30
            // assignment: binary(add) then left, right -> unary -> float
            "assignment",
            "binary_op",
            "integer",
            "unary_op",
            "float",
            // index assign: obj(index) -> obj, key; then key, then value
            "index_assign",
            "index",
            "identifier",
            "integer",
            "boolean",
            "nil",
            // while: condition, then body's break
            "while",
            "boolean",
            "break",
            // do -> expr stmt -> string
            "do",
            "expr_stmt",
            "string",
            // if: condition, then print's three exprs, empty else
            "if",
            "sys_alloc_count",
            "print",
            "sys_exec_count",
            "ffi_module",
            "async_module",
            // expr stmt: call -> field callee -> its obj, then args
            "expr_stmt",
            "call",
            "field",
            "identifier",
            "identifier",
            // return -> function -> its body's return -> its value
            "return",
            "function",
            "return",
            "identifier",
        ]
    );
}
