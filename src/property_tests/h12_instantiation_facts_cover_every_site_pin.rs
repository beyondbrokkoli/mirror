#[test]
fn h12_instantiation_facts_cover_every_site_pin() {
    let source = "\
local acc = 0
local function G()
    acc = acc + 1
    return acc
end
local function F()
    return G()
end
F()
F()
";
    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the union-read pin must analyze clean: {:?}",
        crate::property_tests::diags_text(&shape.diagnostics)
    );

    let root = &shape.instances[&crate::shape::InstanceId::ROOT];
    assert_eq!(root.children.len(), 2, "two F calls, two extents");
    let f_insts: Vec<_> = root.children.values().flatten().copied().collect();
    let f1_call = shape.instances[&f_insts[0]]
        .call
        .expect("each extent names its own call expr");
    let f2_call = shape.instances[&f_insts[1]]
        .call
        .expect("each extent names its own call expr");
    assert_ne!(f1_call, f2_call, "two sites, two call exprs");
    let g_call = *shape.instances[&f_insts[0]]
        .children
        .keys()
        .next()
        .expect("F's extent minted G");

    let g_evt1 = *shape.instances[&f_insts[0]].children[&g_call]
        .iter()
        .next()
        .unwrap();
    let g_evt2 = *shape.instances[&f_insts[1]].children[&g_call]
        .iter()
        .next()
        .unwrap();
    let g_lit = shape.instances[&g_evt1]
        .fn_node
        .expect("the event names its callee literal");
    assert_eq!(
        shape.instances[&g_evt2].fn_node,
        Some(g_lit),
        "both sites' events inline the same literal — monomorphic across sites"
    );
    assert!(
        !root.children.contains_key(&g_call),
        "the nested call's events mint under F's extents, never ROOT — \
         the site law"
    );

    let touched = shape
        .fn_touched
        .get(&g_lit)
        .expect("G's literal records its outer touches");
    assert_eq!(
        touched,
        &std::collections::BTreeSet::from(["acc".to_string()]),
        "G assigns exactly its outer acc"
    );

    let mut expected: std::collections::BTreeMap<
        NodeId,
        std::collections::BTreeSet<NodeId>,
    > = std::collections::BTreeMap::new();
    for rec in shape.instances.values() {
        if let (Some(e), Some(f)) = (rec.call, rec.fn_node) {
            expected.entry(e).or_default().insert(f);
        }
    }
    assert_eq!(
        shape.call_fns, expected,
        "call_fns is the events' projection over the records"
    );
    assert_eq!(
        shape.call_fns[&g_call],
        std::collections::BTreeSet::from([g_lit]),
        "the union read folds every instantiation's literal — one expr, \
         one entry, both sites' events behind it"
    );
}
