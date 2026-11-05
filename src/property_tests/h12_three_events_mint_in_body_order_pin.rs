#[test]
fn h12_three_events_mint_in_body_order_pin() {
    let source = "\
local t = { [0] = 7 }
local function H()
    local m
    return t[0]
end
local function G()
    return H()
end
local function F()
    local q
    local a = G()
    local n = 0
    while n < 9 do
        n = n + 1
        local b = G()
        local p = 0
        while p < 9 do
            p = p + 1
            local c = G()
        end
    end
    return a
end
local z
F()
";
    let ll = compile(source).unwrap_or_else(|(stage, diags)| {
        panic!(
            "the three-event program must lower clean at {}: {}",
            stage.name(),
            diags_text(&diags)
        )
    });
    assert!(
        !ll.contains("@alm_tbl_get_any"),
        "the Int table's reads stay scalar through the three extents"
    );

    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the three-event pin must analyze clean: {:?}",
        crate::property_tests::diags_text(&shape.diagnostics)
    );
    assert_eq!(
        shape.instances.len(),
        8,
        "ROOT + F + three G events + three H events"
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
    assert_eq!(
        f_rec.children.len(),
        4,
        "F's extent minted g_a, g_b, g_c, and the shared h — four call exprs"
    );

    let (h_expr, h_kids) = f_rec
        .children
        .iter()
        .find(|(_, s)| s.len() == 3)
        .expect("the shared body's call minted three events over one pair");
    assert!(
        f_rec.children.contains_key(h_expr),
        "the three-event pair's events mint under the OUTERMOST extent's \
         record (children is the owner's map)"
    );
    let bases: Vec<_> = h_kids
        .iter()
        .map(|k| shape.instances[k].break_base)
        .collect();
    assert_eq!(
        bases,
        vec![0, 1, 2],
        "three events, three depths, ascending ids = mint order = body \
         order — the ordered cursor's exact law; a wrong pick reads the \
         wrong event's base at the Break arm"
    );

    let mut g_bases = Vec::new();
    for (key, set) in f_rec.children.iter() {
        if set.len() == 1 {
            let g_inst = *set.iter().next().unwrap();
            assert_ne!(
                key, h_expr,
                "the G keys are distinct call exprs — one pair per site"
            );
            g_bases.push(shape.instances[&g_inst].break_base);
            assert!(
                shape.instances[&g_inst].children.is_empty(),
                "G's extent minted H, but H rides the OUTERMOST owner's \
                 children — the immediate enclosure stays empty"
            );
        }
    }
    g_bases.sort();
    assert_eq!(
        g_bases,
        vec![0, 1, 2],
        "the three G events mint at the three loop depths too"
    );

    let h_lit = shape.instances[h_kids.iter().next().unwrap()].fn_node;
    let rets: Vec<_> = h_kids
        .iter()
        .map(|k| shape.instances[k].ret.clone())
        .collect();
    assert_eq!(rets[0], rets[1], "one truth pair — the events agree");
    assert_eq!(rets[1], rets[2], "one truth pair — the events agree");
    for k in h_kids {
        assert_eq!(
            shape.instances[k].fn_node, h_lit,
            "every H event inlines the same literal"
        );
    }
    assert!(
        f_rec.children[h_expr].len() == 3,
        "one truth pair, three events — the events are the mint's truth"
    );
}
