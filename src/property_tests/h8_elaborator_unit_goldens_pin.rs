

#[test]
fn h8_elaborator_unit_goldens_pin() {
    use crate::ast::Loc;
    use crate::elaborate;
    let at = Loc { line: 7, col: 3 };

    let for_stmt = elaborate::desugar_numeric_for(
        at,
        "i".into(),
        Expr::Integer {
            value: 0,
            id: NodeId::mint(),
        },
        Expr::Integer {
            value: 4,
            id: NodeId::mint(),
        },
        vec![Stmt::Expr {
            expr: Expr::Integer {
                value: 1,
                id: NodeId::mint(),
            },
            loc: Loc { line: 8, col: 1 },
            id: NodeId::mint(),
        }],
        false,
    );
    let Stmt::Do { body, .. } = &for_stmt else {
        panic!("the numeric-for desugar rides a Do block");
    };
    assert_eq!(body.len(), 3);
    let Stmt::While {
        condition,
        body: wbody,
        ..
    } = &body[2]
    else {
        panic!("the for family's third statement is the While");
    };
    assert!(matches!(
        condition,
        Expr::BinaryOp {
            op: BinOp::LessThan,
            ..
        }
    ));
    assert_eq!(wbody.len(), 2);
    for (what, node) in [
        ("block Do", &for_stmt),
        ("counter decl", (&body[0] as &Stmt)),
        ("latch decl", &body[1]),
        ("While", &body[2]),
        ("inner Do", &wbody[0]),
        ("increment", &wbody[1]),
    ] {
        assert_eq!(
            node.loc(),
            at,
            "the for family's {what} carries the header's stamp"
        );
    }
    let Stmt::Do { body: ubody, .. } = &wbody[0] else {
        panic!("the While body opens with the user's inner Do");
    };
    assert_eq!(ubody[0].loc().line, 8);

    let cells = vec![
        (
            Expr::Identifier {
                name: "o".into(),
                id: NodeId::mint(),
            },
            Expr::Integer {
                value: 0,
                id: NodeId::mint(),
            },
        ),
        (
            Expr::Identifier {
                name: "o".into(),
                id: NodeId::mint(),
            },
            Expr::Integer {
                value: 1,
                id: NodeId::mint(),
            },
        ),
    ];
    let values = vec![
        Expr::Integer {
            value: 5,
            id: NodeId::mint(),
        },
        Expr::Integer {
            value: 6,
            id: NodeId::mint(),
        },
    ];
    let tuple_stmt = elaborate::desugar_tuple_assign(at, cells, values);
    let Stmt::Do { body, .. } = &tuple_stmt else {
        panic!("the tuple desugar rides a Do block");
    };
    assert_eq!(body.len(), 3);
    let Stmt::LocalDecl { names, exprs, .. } = &body[0] else {
        panic!("the tuple family opens with the temp decl");
    };
    assert_eq!(names, &vec!["tuple$t0".to_string(), "tuple$t1".to_string()]);
    assert_eq!(exprs.len(), 2);
    // Literal keys and name objs stay inline, so the decl is the plain
    // two-temp latch; only the store order turns over.
    for (j, stmt) in body[1..].iter().enumerate() {
        let i = 1 - j; // the stores run right-to-left: target 1, then 0
        let Stmt::IndexAssign {
            obj, key, value, ..
        } = stmt
        else {
            panic!("tuple statement {j} is the cell store");
        };
        assert!(matches!(obj, Expr::Identifier { name: n, .. } if n == "o"));
        assert!(matches!(key, Expr::Integer { value: k, .. } if *k == i as i64));
        assert!(
            matches!(value, Expr::Identifier { name: v, .. } if v == &format!("tuple$t{i}")),
            "store {j} reads target {i}'s temp"
        );
        assert_eq!(
            stmt.loc(),
            at,
            "the tuple family's store {j} carries the header's stamp"
        );
    }
    assert_eq!(body[0].loc(), at);
    assert_eq!(tuple_stmt.loc(), at);

    // The dependent-key shape: target 0's key reads a cell, so it
    // latches into tuple$k0 ahead of the value temps.
    let cells = vec![
        (
            Expr::Identifier {
                name: "o".into(),
                id: NodeId::mint(),
            },
            Expr::Index {
                obj: Box::new(Expr::Identifier {
                    name: "o".into(),
                    id: NodeId::mint(),
                }),
                key: Box::new(Expr::Integer {
                    value: 0,
                    id: NodeId::mint(),
                }),
                id: NodeId::mint(),
            },
        ),
        (
            Expr::Identifier {
                name: "o".into(),
                id: NodeId::mint(),
            },
            Expr::Integer {
                value: 1,
                id: NodeId::mint(),
            },
        ),
    ];
    let values = vec![
        Expr::Integer {
            value: 5,
            id: NodeId::mint(),
        },
        Expr::Integer {
            value: 6,
            id: NodeId::mint(),
        },
    ];
    let tuple_stmt = elaborate::desugar_tuple_assign(at, cells, values);
    let Stmt::Do { body, .. } = &tuple_stmt else {
        panic!("the tuple desugar rides a Do block");
    };
    assert_eq!(body.len(), 3);
    let Stmt::LocalDecl { names, exprs, .. } = &body[0] else {
        panic!("the tuple family opens with the temp decl");
    };
    assert_eq!(
        names,
        &vec![
            "tuple$k0".to_string(),
            "tuple$t0".to_string(),
            "tuple$t1".to_string()
        ]
    );
    assert!(matches!(
        &exprs[0],
        Expr::Index { obj, key, .. }
            if matches!(obj.as_ref(), Expr::Identifier { name, .. } if name == "o")
                && matches!(key.as_ref(), Expr::Integer { value: 0, .. })
    ));
    // Target 1's store (literal key, inline) lands first; target 0's
    // store reads the latched key.
    let Stmt::IndexAssign {
        obj, key, value, ..
    } = &body[1]
    else {
        panic!("tuple statement 0 is the cell store");
    };
    assert!(matches!(obj, Expr::Identifier { name: n, .. } if n == "o"));
    assert!(matches!(key, Expr::Integer { value: 1, .. }));
    assert!(matches!(value, Expr::Identifier { name: v, .. } if v == "tuple$t1"));
    let Stmt::IndexAssign {
        obj, key, value, ..
    } = &body[2]
    else {
        panic!("tuple statement 1 is the cell store");
    };
    assert!(matches!(obj, Expr::Identifier { name: n, .. } if n == "o"));
    assert!(matches!(key, Expr::Identifier { name: k, .. } if k == "tuple$k0"));
    assert!(matches!(value, Expr::Identifier { name: v, .. } if v == "tuple$t0"));
    for (what, node) in [
        ("block Do", &tuple_stmt as &Stmt),
        ("temp decl", (&body[0] as &Stmt)),
        ("cell store 0", &body[1]),
        ("cell store 1", &body[2]),
    ] {
        assert_eq!(
            node.loc(),
            at,
            "the tuple family's {what} carries the header's stamp"
        );
    }
}
