use crate::ast::{BinOp, Expr, Loc, NodeId, Stmt};

pub fn desugar_numeric_for(
    at: Loc,
    var: String,
    start: Expr,
    limit: Expr,
    body: Vec<Stmt>,
    inclusive: bool,
) -> Stmt {
    let limit_slot = "for$limit".to_string();
    // The for-limit surface (docs/dialects/for.md): an inclusive value
    // latches `limit + 1` and keeps the `i < for$limit` guard — the
    // comparison, the census fingerprint, and every downstream consumer
    // are identical in both modes, and the +1 folds once at the latch so
    // the latched-once law survives. The guard never changes.
    let latched_limit = if inclusive {
        Expr::BinaryOp {
            op: BinOp::Add,
            left: Box::new(limit),
            right: Box::new(Expr::Integer {
                value: 1,
                id: NodeId::mint(),
            }),
            id: NodeId::mint(),
        }
    } else {
        limit
    };
    let increment = Stmt::Assignment {
        name: var.clone(),
        expr: Expr::BinaryOp {
            op: BinOp::Add,
            left: Box::new(Expr::Identifier {
                name: var.clone(),
                id: NodeId::mint(),
            }),
            right: Box::new(Expr::Integer {
                value: 1,
                id: NodeId::mint(),
            }),
            id: NodeId::mint(),
        },
        loc: at,
        id: NodeId::mint(),
    };
    Stmt::Do {
        body: vec![
            Stmt::LocalDecl {
                names: vec![var.clone()],
                exprs: vec![start],
                loc: at,
                id: NodeId::mint(),
            },
            Stmt::LocalDecl {
                names: vec![limit_slot.clone()],
                exprs: vec![latched_limit],
                loc: at,
                id: NodeId::mint(),
            },
            Stmt::While {
                condition: Expr::BinaryOp {
                    op: BinOp::LessThan,
                    left: Box::new(Expr::Identifier {
                        name: var,
                        id: NodeId::mint(),
                    }),
                    right: Box::new(Expr::Identifier {
                        name: limit_slot,
                        id: NodeId::mint(),
                    }),
                    id: NodeId::mint(),
                },
                body: vec![
                    Stmt::Do {
                        body,
                        loc: at,
                        id: NodeId::mint(),
                    },
                    increment,
                ],
                loc: at,
                id: NodeId::mint(),
            },
        ],
        loc: at,
        id: NodeId::mint(),
    }
}

pub fn desugar_tuple_assign(at: Loc, cells: Vec<(Expr, Expr)>, values: Vec<Expr>) -> Stmt {
    debug_assert_eq!(
        cells.len(),
        values.len(),
        "tuple counts match (parser-enforced)"
    );
    debug_assert!(cells.len() >= 2, "single targets never ride the tuple path");
    let names: Vec<String> = (0..cells.len()).map(|i| format!("tuple$t{i}")).collect();
    let stores = cells
        .into_iter()
        .enumerate()
        .map(|(i, (obj, key))| Stmt::IndexAssign {
            obj,
            key,
            value: Expr::Identifier {
                name: format!("tuple$t{i}"),
                id: NodeId::mint(),
            },
            loc: at,
            id: NodeId::mint(),
        });
    let mut body = vec![Stmt::LocalDecl {
        names,
        exprs: values,
        loc: at,
        id: NodeId::mint(),
    }];
    body.extend(stores);
    Stmt::Do {
        body,
        loc: at,
        id: NodeId::mint(),
    }
}
