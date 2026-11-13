#[test]
#[should_panic(expected = "tuple-assign attribution drift")]
fn h8_census_trips_on_name_store_wrong_line() {
    use crate::ast::Loc;
    let at = |line: u32| Loc { line, col: 1 };
    // x, y = y, x with the second name store stamped off-header: the
    // census must read Assignment stores for the attribution law to
    // hold on name targets — the pinned-arg rejection lands on the
    // header's line through exactly this arm.
    let ast = vec![Stmt::Do {
        body: vec![
            Stmt::LocalDecl {
                names: vec!["tuple$t0".into(), "tuple$t1".into()],
                exprs: vec![
                    Expr::Identifier {
                        name: "y".into(),
                        id: NodeId::mint(),
                    },
                    Expr::Identifier {
                        name: "x".into(),
                        id: NodeId::mint(),
                    },
                ],
                loc: at(2),
                id: NodeId::mint(),
            },
            Stmt::Assignment {
                name: "y".into(),
                expr: Expr::Identifier {
                    name: "tuple$t1".into(),
                    id: NodeId::mint(),
                },
                loc: at(2),
                id: NodeId::mint(),
            },
            Stmt::Assignment {
                name: "x".into(),
                expr: Expr::Identifier {
                    name: "tuple$t0".into(),
                    id: NodeId::mint(),
                },
                loc: at(9),
                id: NodeId::mint(),
            },
        ],
        loc: at(2),
        id: NodeId::mint(),
    }];
    analysis::build_context(&ast);
}

#[test]
#[should_panic(expected = "does not read temp")]
fn h8_census_trips_on_name_store_left_to_right() {
    use crate::ast::Loc;
    let at = |line: u32| Loc { line, col: 1 };
    // The pre-T1 order: name stores emitted left-to-right, each
    // reading its own temp. For duplicate name targets that order
    // lets the LAST value win; the judge (and T1) store right-to-left
    // so the FIRST wins — d, d = 1, 2 must print 1. The census holds
    // the store order on name targets exactly as on cells.
    let ast = vec![Stmt::Do {
        body: vec![
            Stmt::LocalDecl {
                names: vec!["tuple$t0".into(), "tuple$t1".into()],
                exprs: vec![
                    Expr::Integer {
                        value: 1,
                        id: NodeId::mint(),
                    },
                    Expr::Integer {
                        value: 2,
                        id: NodeId::mint(),
                    },
                ],
                loc: at(2),
                id: NodeId::mint(),
            },
            Stmt::Assignment {
                name: "d".into(),
                expr: Expr::Identifier {
                    name: "tuple$t0".into(),
                    id: NodeId::mint(),
                },
                loc: at(2),
                id: NodeId::mint(),
            },
            Stmt::Assignment {
                name: "d".into(),
                expr: Expr::Identifier {
                    name: "tuple$t1".into(),
                    id: NodeId::mint(),
                },
                loc: at(2),
                id: NodeId::mint(),
            },
        ],
        loc: at(2),
        id: NodeId::mint(),
    }];
    analysis::build_context(&ast);
}
