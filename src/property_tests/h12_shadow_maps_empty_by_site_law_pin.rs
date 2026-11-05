#[test]
fn h12_shadow_maps_empty_by_site_law_pin() {
    let source = "\
do
    local data = { [0] = 42 }
    local map = { [0] = 0 }
    local function ReadNested(i)
        return data[map[i]]
    end
    print(ReadNested(0))
end
do
    local store = { 100, 200 }
    local function Get(idx)
        return store[idx]
    end
    local function Outer(idx)
        return Get(idx) + 1
    end
    print(Outer(0))
end
";
    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the site-law pin must analyze clean: {:?}",
        crate::property_tests::diags_text(&shape.diagnostics)
    );

    assert_eq!(
        shape.instances.len(),
        4,
        "ROOT plus one instance per expansion event"
    );
    let root = &shape.instances[&crate::shape::InstanceId::ROOT];
    assert_eq!(
        root.children.len(),
        2,
        "the two top-level self-pairs mint ROOT's children"
    );
    let mut kids = root.children.values().flatten().copied();
    let (a, b) = (kids.next().unwrap(), kids.next().unwrap());
    let (rn_inst, out_inst) = if shape.instances[&a].children.is_empty() {
        (a, b)
    } else {
        (b, a)
    };
    let rn = &shape.instances[&rn_inst];
    let out = &shape.instances[&out_inst];
    assert!(
        rn.children.is_empty(),
        "ReadNested's extent mints no expansions"
    );
    assert_eq!(out.children.len(), 1, "Outer's extent minted exactly Get");

    assert_eq!(
        rn.access_elems.len(),
        2,
        "data's and map's elem faces ride ReadNested's extent"
    );
    assert!(
        rn.access_elems
            .values()
            .all(|ty| *ty == crate::shape::Ty::Int),
        "both bases are Int tables"
    );
    assert_eq!(
        rn.index_key_lanes.len(),
        2,
        "data's dynamic key map[i] AND map's dynamic key i each carry a \
         recorded lane — a dynamic key inside a dynamic key is two lanes"
    );
    assert!(
        rn.index_key_lanes
            .values()
            .all(|lane| *lane == crate::shape::KeyLane::Int),
        "an Int key rides the Int lane"
    );

    assert_eq!(
        out.access_elems.len(),
        1,
        "store's elem face rides Outer's extent, not Get's"
    );
    assert!(
        out.access_elems
            .values()
            .all(|ty| *ty == crate::shape::Ty::Int),
        "store is an Int table"
    );
    let get_inst = *out.children.values().next().unwrap().iter().next().unwrap();
    let get_rec = &shape.instances[&get_inst];
    assert!(
        get_rec.access_elems.is_empty()
            && get_rec.index_key_lanes.is_empty()
            && get_rec.creation_envs.is_empty()
            && get_rec.closure_calls.is_empty()
            && get_rec.children.is_empty(),
        "the immediate enclosure owns NOTHING — its shadow maps stay empty by law"
    );
}
