#[test]
fn h12_local_types_key_the_outermost_extent_pin() {
    let source = "\
local function G()
    local m
    return 0
end
local function F()
    local q
    return G()
end
local z
F()
F()
";
    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the bare-decl pin must analyze clean: {:?}",
        crate::property_tests::diags_text(&shape.diagnostics)
    );

    let insts_of = |name: &str| -> Vec<crate::shape::InstanceId> {
        let mut insts: Vec<crate::shape::InstanceId> = shape
            .local_types
            .iter()
            .filter(|((_, _, n), _)| n == name)
            .map(|((_, i, _), _)| *i)
            .collect();
        insts.sort();
        insts
    };

    let m_insts = insts_of("m");
    assert_eq!(m_insts.len(), 2, "m's type records once per F extent");
    assert_ne!(m_insts[0], m_insts[1], "one record per extent");
    let root_rec = &shape.instances[&crate::shape::InstanceId::ROOT];
    for k in &m_insts {
        let rec = shape
            .instances
            .get(k)
            .expect("the key instance is materialized");
        let c = rec
            .call
            .expect("a decl inside an expansion keys an event instance");
        assert!(
            root_rec
                .children
                .get(&c)
                .is_some_and(|s| s.contains(k)),
            "the key instance is a TOP-LEVEL event (ROOT's children hold \
             its call and count this instance among the call's mints) — \
             the OUTERMOST extent, never the immediate enclosure's G event"
        );
        assert!(
            !rec.children.is_empty(),
            "the immediate enclosure's own G event hangs UNDER the key \
             record (the site law: the owner is first(), never last())"
        );
    }

    let q_insts = insts_of("q");
    assert_eq!(q_insts.len(), 2, "F's own bare decl records per extent");
    assert!(
        q_insts
            .iter()
            .all(|k| shape.instances[k].call.is_some()),
        "F's body is checked inside F's expansion — an event instance"
    );

    assert_eq!(
        insts_of("z"),
        vec![crate::shape::InstanceId::ROOT],
        "the top level's bare decl keys ROOT"
    );
}
