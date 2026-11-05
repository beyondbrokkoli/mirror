#[test]
fn h12_recursion_guard_reads_the_instance_chain_pin() {
    let source = "\
local function H()
    return 1
end
local function G()
    return H()
end
local function F()
    local n = 0
    while n < 2 do
        n = n + 1
        local b = G()
    end
    return n
end
print(F())
";
    let ll = compile(source).unwrap_or_else(|(stage, diags)| {
        panic!(
            "the deep chain program must lower clean at {}: {}",
            stage.name(),
            diags_text(&diags)
        )
    });
    assert!(
        !ll.contains("@alm_tbl_get_any"),
        "the chain program touches no tables at all"
    );

    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the chain pin must analyze clean: {:?}",
        diags_text(&shape.diagnostics)
    );

    let fn_lit = |stmt: usize| match &ast[stmt] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("the statement declares a fn"),
    };
    let fn_body = |stmt: usize| match &ast[stmt] {
        Stmt::LocalDecl { exprs, .. } => match &exprs[0] {
            Expr::Function { body, .. } => body,
            _ => panic!("the local binds a fn literal"),
        },
        _ => panic!("the statement declares a fn"),
    };
    let h_lit = fn_lit(0);
    let g_lit = fn_lit(1);
    let f_lit = fn_lit(2);
    let f_call = match &ast[3] {
        Stmt::Print { exprs, .. } => exprs[0].id(),
        _ => panic!("the statement prints one call"),
    };
    let g_call = match &fn_body(2)[1] {
        Stmt::While { body, .. } => match &body[1] {
            Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
            _ => panic!("F's loop calls G in a value position"),
        },
        _ => panic!("F's body carries the while"),
    };
    let h_call = match &fn_body(1)[0] {
        Stmt::Return {
            value: Some(call @ Expr::Call { .. }),
            ..
        } => call.id(),
        _ => panic!("G's body is one H call"),
    };

    let mut chain: Vec<crate::shape::InstanceId> = Vec::new();
    let mut cursor: std::collections::BTreeMap<
        (crate::shape::InstanceId, NodeId),
        usize,
    > = std::collections::BTreeMap::new();

    let owner = chain.first().copied().unwrap_or(crate::shape::InstanceId::ROOT);
    let kids = &shape.instances[&owner].children[&f_call];
    let pair_fn: std::collections::BTreeSet<_> = kids
        .iter()
        .map(|k| shape.instances[k].fn_node)
        .collect();
    assert_eq!(
        pair_fn,
        std::collections::BTreeSet::from([Some(f_lit)]),
        "the pair tier answers identically off any kid (level F)"
    );
    let k = *cursor.entry((owner, f_call)).or_insert(0);
    let k_f = *kids.iter().nth(k).expect("the cursor picks in mint order");
    *cursor.get_mut(&(owner, f_call)).unwrap() += 1;
    chain.push(k_f);
    assert_eq!(
        shape.instances[&k_f].break_base,
        0,
        "the event tier: F's record carries the top level's base"
    );
    let guard_input: std::collections::BTreeSet<_> = chain
        .iter()
        .map(|i| shape.instances[i].fn_node)
        .collect();
    assert_eq!(
        guard_input,
        std::collections::BTreeSet::from([Some(f_lit)]),
        "the guard's input off the records: exactly the active fn set"
    );

    let owner = chain.first().copied().unwrap_or(crate::shape::InstanceId::ROOT);
    let kids = &shape.instances[&owner].children[&g_call];
    let pair_fn: std::collections::BTreeSet<_> = kids
        .iter()
        .map(|k| shape.instances[k].fn_node)
        .collect();
    assert_eq!(
        pair_fn,
        std::collections::BTreeSet::from([Some(g_lit)]),
        "the pair tier answers identically off any kid (level G)"
    );
    let k = *cursor.entry((owner, g_call)).or_insert(0);
    let k_g = *kids.iter().nth(k).expect("the cursor picks in mint order");
    *cursor.get_mut(&(owner, g_call)).unwrap() += 1;
    chain.push(k_g);
    assert_eq!(
        shape.instances[&k_g].break_base,
        1,
        "the event tier: G's record carries F's while's absolute base"
    );
    let guard_input: std::collections::BTreeSet<_> = chain
        .iter()
        .map(|i| shape.instances[i].fn_node)
        .collect();
    assert_eq!(
        guard_input,
        std::collections::BTreeSet::from([Some(f_lit), Some(g_lit)]),
        "the guard's input off the records: exactly the active fn set"
    );

    let owner = chain.first().copied().unwrap_or(crate::shape::InstanceId::ROOT);
    assert_eq!(
        owner, k_f,
        "the owner is the OUTERMOST extent's instance, never the \
         immediate enclosure (kG)"
    );
    assert!(
        shape.instances[&k_g].children.is_empty(),
        "the immediate enclosure owns nothing — the site law"
    );
    let kids = &shape.instances[&owner].children[&h_call];
    let pair_fn: std::collections::BTreeSet<_> = kids
        .iter()
        .map(|k| shape.instances[k].fn_node)
        .collect();
    assert_eq!(
        pair_fn,
        std::collections::BTreeSet::from([Some(h_lit)]),
        "the pair tier answers identically off any kid (level H)"
    );
    let k = *cursor.entry((owner, h_call)).or_insert(0);
    let k_h = *kids.iter().nth(k).expect("the cursor picks in mint order");
    *cursor.get_mut(&(owner, h_call)).unwrap() += 1;
    chain.push(k_h);
    assert_eq!(
        shape.instances[&k_h].break_base,
        1,
        "the event tier: H's record inherits G's absolute base"
    );
    let guard_input: std::collections::BTreeSet<_> = chain
        .iter()
        .map(|i| shape.instances[i].fn_node)
        .collect();
    assert_eq!(
        guard_input,
        std::collections::BTreeSet::from([Some(f_lit), Some(g_lit), Some(h_lit)]),
        "the no-re-derivation law, stated at the deepest level: the \
         chain's fn_nodes are exactly the active fn set the old \
         inline_stack would have held — a recursive call to F, G, or H \
         from H's body would fire the guard reading these records alone"
    );

    for expected in [k_h, k_g, k_f] {
        assert_eq!(chain.pop(), Some(expected), "the chain pops in LIFO order");
    }
    assert!(chain.is_empty(), "back at top level, the chain is empty");
}
