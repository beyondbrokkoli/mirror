#[test]
fn h8_tuple_name_target_goldens_pin() {
    use crate::ast::{CtorKey, Loc};
    use crate::elaborate::{self, TupleTarget};
    let at = Loc { line: 7, col: 3 };
    let ident = |name: &str| Expr::Identifier {
        name: name.into(),
        id: NodeId::mint(),
    };
    let int = |value: i64| Expr::Integer {
        value,
        id: NodeId::mint(),
    };
    let cell = |obj: Expr, key: Expr| TupleTarget::Cell { obj, key };
    let name = |n: &str| TupleTarget::Name(n.into());
    let is_ident = |e: &Expr, want: &str| matches!(e, Expr::Identifier { name, .. } if name == want);
    let open = |stmt: &Stmt| -> (Vec<String>, Vec<Stmt>) {
        let Stmt::Do { body, .. } = stmt else {
            panic!("the tuple desugar rides a Do block");
        };
        let Stmt::LocalDecl { names, .. } = &body[0] else {
            panic!("the tuple family opens with the temp decl");
        };
        for node in body.iter().chain(std::iter::once(stmt)) {
            assert_eq!(node.loc(), at, "every node carries the header's stamp");
        }
        (names.clone(), body[1..].to_vec())
    };
    let temps = |ns: &[&str]| ns.iter().map(|n| n.to_string()).collect::<Vec<_>>();

    // x, y = y, x — names own no prefix: the decl is the value tail
    // alone, and the rebinds land right-to-left.
    let swap = elaborate::desugar_tuple_assign(
        at,
        vec![name("x"), name("y")],
        vec![ident("y"), ident("x")],
    );
    let (names, stores) = open(&swap);
    assert_eq!(names, temps(&["tuple$t0", "tuple$t1"]));
    assert!(matches!(
        &stores[0],
        Stmt::Assignment { name, expr, .. } if name == "y" && is_ident(expr, "tuple$t1")
    ));
    assert!(matches!(
        &stores[1],
        Stmt::Assignment { name, expr, .. } if name == "x" && is_ident(expr, "tuple$t0")
    ));
    analysis::build_context(std::slice::from_ref(&swap));

    // b[j], j = 20, j + 1 — the name store lands before the cell
    // store, so the key latches the old j (the judge prints 4 20 0).
    let capture = elaborate::desugar_tuple_assign(
        at,
        vec![cell(ident("b"), ident("j")), name("j")],
        vec![
            int(20),
            Expr::BinaryOp {
                op: BinOp::Add,
                left: Box::new(ident("j")),
                right: Box::new(int(1)),
                id: NodeId::mint(),
            },
        ],
    );
    let (names, stores) = open(&capture);
    assert_eq!(names, temps(&["tuple$k0", "tuple$t0", "tuple$t1"]));
    assert!(matches!(
        &stores[0],
        Stmt::Assignment { name, expr, .. } if name == "j" && is_ident(expr, "tuple$t1")
    ));
    assert!(matches!(
        &stores[1],
        Stmt::IndexAssign { obj, key, value, .. }
            if is_ident(obj, "b") && is_ident(key, "tuple$k0") && is_ident(value, "tuple$t0")
    ));
    analysis::build_context(std::slice::from_ref(&capture));

    // i, a[i] = i + 1, 20 — the mirror: the cell store lands first,
    // the name rebinds after it, and the key stays inline.
    let classic = elaborate::desugar_tuple_assign(
        at,
        vec![name("i"), cell(ident("a"), ident("i"))],
        vec![
            Expr::BinaryOp {
                op: BinOp::Add,
                left: Box::new(ident("i")),
                right: Box::new(int(1)),
                id: NodeId::mint(),
            },
            int(20),
        ],
    );
    let (names, stores) = open(&classic);
    assert_eq!(names, temps(&["tuple$t0", "tuple$t1"]));
    assert!(matches!(
        &stores[0],
        Stmt::IndexAssign { obj, key, value, .. }
            if is_ident(obj, "a") && is_ident(key, "i") && is_ident(value, "tuple$t1")
    ));
    assert!(matches!(
        &stores[1],
        Stmt::Assignment { name, expr, .. } if name == "i" && is_ident(expr, "tuple$t0")
    ));
    analysis::build_context(std::slice::from_ref(&classic));

    // t[0], t = 5, u — the obj itself rebinds first, so it latches:
    // the store rides the old table (the judge prints u's own cells).
    let displace = elaborate::desugar_tuple_assign(
        at,
        vec![cell(ident("t"), int(0)), name("t")],
        vec![int(5), ident("u")],
    );
    let (names, stores) = open(&displace);
    assert_eq!(names, temps(&["tuple$o0", "tuple$t0", "tuple$t1"]));
    assert!(matches!(
        &stores[0],
        Stmt::Assignment { name, expr, .. } if name == "t" && is_ident(expr, "tuple$t1")
    ));
    assert!(matches!(
        &stores[1],
        Stmt::IndexAssign { obj, key, value, .. }
            if is_ident(obj, "tuple$o0")
                && matches!(key, Expr::Integer { value: 0, .. })
                && is_ident(value, "tuple$t0")
    ));
    analysis::build_context(std::slice::from_ref(&displace));

    // o[0], ({7})[0] = 5, 6 — a ctor prefix mints a table and runs its
    // entries inside the judge's pre-store window, so it latches.
    let ctor = elaborate::desugar_tuple_assign(
        at,
        vec![
            cell(ident("o"), int(0)),
            cell(
                Expr::TableCtor {
                    entries: vec![(CtorKey::Const(0), int(7))],
                    line: 7,
                    id: NodeId::mint(),
                },
                int(0),
            ),
        ],
        vec![int(5), int(6)],
    );
    let (names, stores) = open(&ctor);
    assert_eq!(names, temps(&["tuple$o1", "tuple$t0", "tuple$t1"]));
    assert!(matches!(
        &stores[0],
        Stmt::IndexAssign { obj, value, .. }
            if is_ident(obj, "tuple$o1") && is_ident(value, "tuple$t1")
    ));
    assert!(matches!(
        &stores[1],
        Stmt::IndexAssign { obj, value, .. } if is_ident(obj, "o") && is_ident(value, "tuple$t0")
    ));
    analysis::build_context(std::slice::from_ref(&ctor));
}
