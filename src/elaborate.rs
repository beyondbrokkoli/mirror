use crate::ast::{BinOp, Expr, Loc, NodeId, Stmt, UnOp};

pub fn desugar_numeric_for(
    at: Loc,
    var: String,
    start: Expr,
    limit: Expr,
    body: Vec<Stmt>,
    inclusive: bool,
) -> Stmt {
    let limit_slot = "for$limit".to_string();
    // The for-limit surface: an inclusive value latches `limit + 1`
    // and keeps the `i < for$limit` guard — the comparison, the census
    // fingerprint, and every downstream consumer are identical in both
    // modes, and the +1 folds once at the latch so the latched-once law
    // survives. The guard never changes.
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

/// A target prefix (obj or key) latches into a temp iff it reads a
/// cell, calls, or samples the system counters — the only things the
/// judge's pre-store evaluation window can observe. Everything else
/// (literals, names, arithmetic over names) is frozen across the
/// window: the expansion's stores write cells, never names, so an
/// unlatched prefix reads the same value at store time as it would
/// have at latch time. Literal keys especially stay inline — the
/// static-key laws key on them.
fn prefix_needs_latch(e: &Expr) -> bool {
    match e {
        Expr::Index { .. }
        | Expr::Field { .. }
        | Expr::Call { .. }
        | Expr::SysAllocCount { .. }
        | Expr::SysExecCount { .. } => true,
        Expr::UnaryOp { op: UnOp::Len, .. } => true,
        Expr::UnaryOp { expr, .. } => prefix_needs_latch(expr),
        Expr::BinaryOp { left, right, .. } => prefix_needs_latch(left) || prefix_needs_latch(right),
        _ => false,
    }
}

/// The judge's tuple-assignment law (Lua 5.1 manual §2.4.3; probed on
/// luajit 2.1.1794214635, lua5.4 agreeing): every target's table and
/// key evaluates before any store, then the stores land
/// right-to-left. The temps below compose one decl whose left-to-right
/// evaluation is that order — latch exprs in target order, then the
/// RHS values — followed by the stores in reverse target order.
pub fn desugar_tuple_assign(at: Loc, cells: Vec<(Expr, Expr)>, values: Vec<Expr>) -> Stmt {
    debug_assert_eq!(
        cells.len(),
        values.len(),
        "tuple counts match (parser-enforced)"
    );
    debug_assert!(cells.len() >= 2, "single targets never ride the tuple path");
    let n = cells.len();
    let mut names: Vec<String> = Vec::with_capacity(2 * n + values.len());
    let mut exprs: Vec<Expr> = Vec::with_capacity(2 * n + values.len());
    // Inline prefixes ride the store unmoved; latched prefixes move
    // into the decl. Each original node lands in the tree exactly once.
    let mut inline_objs: Vec<Option<Expr>> = vec![None; n];
    let mut inline_keys: Vec<Option<Expr>> = vec![None; n];
    for (i, (obj, key)) in cells.into_iter().enumerate() {
        if prefix_needs_latch(&obj) {
            names.push(format!("tuple$o{i}"));
            exprs.push(obj);
        } else {
            inline_objs[i] = Some(obj);
        }
        if prefix_needs_latch(&key) {
            names.push(format!("tuple$k{i}"));
            exprs.push(key);
        } else {
            inline_keys[i] = Some(key);
        }
    }
    for (i, value) in values.into_iter().enumerate() {
        names.push(format!("tuple$t{i}"));
        exprs.push(value);
    }
    let temp = |name: String| Expr::Identifier {
        name,
        id: NodeId::mint(),
    };
    let stores = (0..n).rev().map(|i| {
        let obj = inline_objs[i]
            .take()
            .unwrap_or_else(|| temp(format!("tuple$o{i}")));
        let key = inline_keys[i]
            .take()
            .unwrap_or_else(|| temp(format!("tuple$k{i}")));
        Stmt::IndexAssign {
            obj,
            key,
            value: temp(format!("tuple$t{i}")),
            loc: at,
            id: NodeId::mint(),
        }
    });
    let mut body = vec![Stmt::LocalDecl {
        names,
        exprs,
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
