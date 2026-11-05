

#[test]
fn h12_instance_mint_law_pin() {
    let source = "\
local function H() return 5 end
local function G() return H() end
local function F() return G() + G() end
print(F())
print(F())
";
    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "H12 pin source must analyze clean: {:?}",
        crate::property_tests::diags_text(&shape.diagnostics)
    );

    let call_of_print = |stmt: usize| match &ast[stmt] {
        Stmt::Print { exprs, .. } => &exprs[0],
        _ => panic!("the statement prints one call"),
    };
    let f1 = call_of_print(3).id();
    let f2 = call_of_print(4).id();
    let fn_body = |stmt: usize| match &ast[stmt] {
        Stmt::LocalDecl { exprs, .. } => match &exprs[0] {
            Expr::Function { body, .. } => body,
            _ => panic!("the local binds a fn literal"),
        },
        _ => panic!("the statement declares a fn"),
    };
    let f_body = fn_body(2);
    let (g1, g2) = match &f_body[0] {
        Stmt::Return {
            value: Some(Expr::BinaryOp { left, right, .. }),
            ..
        } => (left.as_ref(), right.as_ref()),
        _ => panic!("F's body returns the sum of two calls"),
    };
    let (g1, g2, h) = (
        g1.id(),
        g2.id(),
        match &fn_body(1)[0] {
            Stmt::Return {
                value: Some(call @ Expr::Call { .. }),
                ..
            } => call.id(),
            _ => panic!("G's body returns one call"),
        },
    );

    let root = &shape.instances[&crate::shape::InstanceId::ROOT];
    let only_child =
        |owner_children: &std::collections::BTreeMap<
            NodeId,
            std::collections::BTreeSet<crate::shape::InstanceId>,
        >,
         call: NodeId| { *owner_children[&call].iter().next().unwrap() };
    let kf1 = only_child(&root.children, f1);
    let kf2 = only_child(&root.children, f2);
    let kg1 = only_child(&shape.instances[&kf1].children, g1);
    let kg2 = only_child(&shape.instances[&kf1].children, g2);
    let (kg3, kg4) = (
        only_child(&shape.instances[&kf2].children, g1),
        only_child(&shape.instances[&kf2].children, g2),
    );
    let h_under_f1 = {
        let mut v: Vec<_> = shape.instances[&kf1].children[&h].iter().copied().collect();
        v.sort();
        (v[0], v[1])
    };
    let (kh1, kh2) = h_under_f1;
    let h_under_f2 = {
        let mut v: Vec<_> = shape.instances[&kf2].children[&h].iter().copied().collect();
        v.sort();
        (v[0], v[1])
    };
    let (kh3, kh4) = h_under_f2;
    for kg in [kg1, kg2, kg3, kg4] {
        assert!(
            shape.instances[&kg].children.is_empty(),
            "a nested instance owns nothing — the site law keys against the outermost"
        );
    }

    assert_eq!(
        shape.instances.len(),
        11,
        "ten expansion events mint ten instances plus ROOT"
    );

    let order = [kf1, kg1, kh1, kg2, kh2, kf2, kg3, kh3, kg4, kh4];
    for w in order.windows(2) {
        assert!(
            w[0] < w[1],
            "instance ids draw in entry order: {w:?} out of order"
        );
    }
    assert!(
        order.iter().all(|&k| k > crate::shape::InstanceId::ROOT),
        "ROOT (0) is never drawn from the mint"
    );

    assert_ne!(
        kh1, kh3,
        "the same call expr at two sites mints two instances"
    );
    assert_eq!(
        shape.instances[&kf1].children[&h].len(),
        2,
        "one pair, two events under one owner — children is set-valued"
    );
    assert_eq!(shape.instances[&kf2].children[&h].len(), 2);
    let h_events: std::collections::BTreeSet<_> = shape
        .instances
        .iter()
        .filter(|(_, r)| r.call == Some(h))
        .map(|(&k, _)| k)
        .collect();
    let owned: std::collections::BTreeSet<_> = shape.instances[&kf1].children[&h]
        .union(&shape.instances[&kf2].children[&h])
        .copied()
        .collect();
    assert_eq!(
        h_events, owned,
        "every H event mints under the OUTERMOST expansion's record — \
         the site law's owner face (children is the owner's map)"
    );

    for (&k, rec) in &shape.instances {
        if k == crate::shape::InstanceId::ROOT {
            assert!(rec.call.is_none() && rec.fn_node.is_none() && rec.ret.is_none());
            continue;
        }
        assert!(rec.call.is_some(), "a minted record names its call");
        let callee = rec.fn_node.expect("a minted record names its callee");
        assert!(
            shape.fn_defs.contains_key(&callee),
            "the record's callee is a real fn literal"
        );
    }
    let h_lit = match &ast[0] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("H's literal is the first declaration"),
    };
    for kh in [kh1, kh2, kh3, kh4] {
        let rec = &shape.instances[&kh];
        assert_eq!(
            rec.fn_node,
            Some(h_lit),
            "every H event inlines the same literal"
        );
        assert_eq!(rec.call, Some(h), "every H event names the same call node");
        assert!(!rec.body.is_empty(), "the record owns the body");
        assert_eq!(rec.body.len(), 1, "H's body is one statement");
    }
}
