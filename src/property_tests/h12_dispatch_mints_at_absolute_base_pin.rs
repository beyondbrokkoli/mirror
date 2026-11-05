#[test]
fn h12_dispatch_mints_at_absolute_base_pin() {
    let source = "\
local function MakeAdder(seed)
    local function Apply(x) return x + seed end
    return Apply
end
local add7 = MakeAdder(7)
local box = { op = add7 }
local acc = 0
while acc < 20 do
    acc = box.op(acc)
end
print(acc)
";
    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the dispatch pin must analyze clean: {:?}",
        crate::property_tests::diags_text(&shape.diagnostics)
    );

    let root = &shape.instances[&crate::shape::InstanceId::ROOT];
    assert_eq!(
        root.children.len(),
        2,
        "MakeAdder's call and the dispatch mint ROOT's children"
    );
    let kids: Vec<_> = root.children.values().flatten().copied().collect();

    let app_inst = *kids
        .iter()
        .find(|&&k| shape.instances[&k].env_face.is_some())
        .expect("the record-field dispatch records a call-through env face");
    let maker_inst = *kids
        .iter()
        .find(|&&k| shape.instances[&k].env_face.is_none())
        .expect("the plain factory call carries no call-through face");
    let app_rec = &shape.instances[&app_inst];

    assert_eq!(
        app_rec.break_base, 1,
        "the dispatch mints at absolute base 1 — inside the loop"
    );
    assert_eq!(
        shape.instances[&maker_inst].break_base, 0,
        "the factory's call mints at the top level — base 0"
    );

    let face = app_rec.env_face.as_ref().unwrap();
    assert_eq!(
        face.get("seed"),
        Some(&crate::shape::Ty::Int),
        "the env face names Apply's capture"
    );

    let boxop = app_rec
        .call
        .expect("the dispatch's record names its call expr");
    assert!(
        root.closure_calls.contains(&boxop),
        "the Field dispatch rides closure_calls' pair truth"
    );
    assert!(
        root.children.contains_key(&boxop),
        "the top-level dispatch's events mint under ROOT — the self-pair \
         owner convention (children is the owner's map)"
    );

    assert!(
        root.creation_envs.is_empty(),
        "a callee-position read never mints — creation is the value read's land"
    );
    let apply_lit = app_rec.fn_node.expect("the dispatch inlines Apply");
    let maker_rec = &shape.instances[&maker_inst];
    let apply_creations: Vec<_> = maker_rec.creation_envs.values().copied().collect();
    assert_eq!(
        apply_creations,
        vec![apply_lit],
        "Apply's escape read (`return Apply`) mints the factory extent's \
         creation env, naming Apply's literal"
    );
}
