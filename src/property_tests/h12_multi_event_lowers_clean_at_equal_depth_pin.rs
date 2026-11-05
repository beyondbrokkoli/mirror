#[test]
fn h12_multi_event_lowers_clean_at_equal_depth_pin() {
    let source = "\
local t = { [0] = 7 }
local function H()
    return t[0]
end
local function G()
    return H()
end
local function F()
    local a = G()
    local b = G()
    return a + b
end
print(F())
";
    let ll = compile(source).unwrap_or_else(|(stage, diags)| {
        panic!(
            "the equal-depth multi-event program must lower clean at {}: {}",
            stage.name(),
            diags_text(&diags)
        )
    });
    assert!(
        !ll.contains("@alm_tbl_get_any"),
        "the Int table's reads stay scalar through the shared body"
    );

    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the equal-depth twin must analyze clean: {:?}",
        crate::property_tests::diags_text(&shape.diagnostics)
    );
    assert_eq!(
        shape.instances.len(),
        6,
        "ROOT + F + two G events + two H events"
    );
    let root = &shape.instances[&crate::shape::InstanceId::ROOT];
    let f_inst = *root
        .children
        .values()
        .next()
        .unwrap()
        .iter()
        .next()
        .unwrap();
    let f_rec = &shape.instances[&f_inst];
    let (_, h_kids) = f_rec
        .children
        .iter()
        .find(|(_, s)| s.len() == 2)
        .expect("the shared body's call mints two events over one pair");
    let bases: Vec<_> = h_kids
        .iter()
        .map(|k| shape.instances[k].break_base)
        .collect();
    assert_eq!(
        bases,
        vec![0, 0],
        "both events mint at the same absolute base — the pick is sound here"
    );
}
