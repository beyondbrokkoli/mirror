#[test]
fn h12_break_base_is_per_event_pin() {
    let source = "\
local t = { [0] = 7 }
local acc = 0
local function H()
    local m
    acc = acc + 1
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
        local c = G()
    end
    return a
end
local z
F()
";
    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the per-event pin must analyze clean: {:?}",
        crate::property_tests::diags_text(&shape.diagnostics)
    );

    assert_eq!(
        shape.instances.len(),
        6,
        "ROOT + F + two G events + two H events"
    );

    let root = &shape.instances[&crate::shape::InstanceId::ROOT];
    assert_eq!(
        root.children.len(),
        1,
        "F's top-level call is ROOT's only child key"
    );
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
        3,
        "F's extent minted g1, g2, and h — three call exprs"
    );
    assert_eq!(
        f_rec.break_base, 0,
        "F itself expands at the top level — absolute base 0"
    );

    let (&h_expr, h_kids) = f_rec
        .children
        .iter()
        .find(|(_, s)| s.len() == 2)
        .expect("the shared body's call mints two events over one pair");
    assert!(
        f_rec.children.contains_key(&h_expr),
        "the multi-event pair's events mint under the OUTERMOST extent's \
         record (children is the owner's map)"
    );
    let mut bases = h_kids.iter().map(|k| shape.instances[k].break_base);
    let (b1, b2) = (bases.next().unwrap(), bases.next().unwrap());
    assert_eq!(
        (b1, b2),
        (0, 1),
        "the events' absolute bases diverge by exactly the while — break_base \
         is per-EVENT; 7b's pick must be the mint-ordered cursor, an any-kid \
         read binds a caller's loop at the Break arm"
    );

    let h_lit = shape.instances[h_kids.iter().next().unwrap()].fn_node;
    let rets: Vec<_> = h_kids
        .iter()
        .map(|k| shape.instances[k].ret.clone())
        .collect();
    assert_eq!(rets[0], rets[1], "one truth pair, two events — rets agree");
    for k in h_kids {
        let rec = &shape.instances[k];
        assert_eq!(rec.fn_node, h_lit, "every H event inlines the same literal");
    }
    assert!(
        h_kids.len() == 2,
        "one truth pair, TWO events — children carry BOTH while the pair \
         faces stay monomorphic"
    );

    let g_insts: Vec<_> = f_rec
        .children
        .iter()
        .filter(|(_, s)| s.len() == 1)
        .flat_map(|(_, s)| s.iter().copied())
        .collect();
    assert_eq!(g_insts.len(), 2, "two G events, one per call expr");
    for g in g_insts {
        assert!(
            shape.instances[&g].children.is_empty(),
            "G's extent minted H, but H rides the OUTERMOST owner's children — \
             the immediate enclosure stays empty"
        );
    }

    assert_eq!(
        f_rec.access_elems.len(),
        1,
        "t's read inside H's body pairs with F's extent"
    );
    assert!(
        f_rec
            .access_elems
            .values()
            .all(|ty| *ty == crate::shape::Ty::Int),
        "t is an Int table"
    );
}
