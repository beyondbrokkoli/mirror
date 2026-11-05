#[test]
fn h12_lowerer_consumes_the_records_pin() {
    let source = "\
local t = { [0] = 7 }
local acc = 0
local function K()
    acc = acc + 10
    return 0
end
local function H()
    local m
    m = t[0]
    local c = K()
    local n = 0
    while n < 3 do
        n = n + 1
        if n == 2 then
            break
        end
        acc = acc + 1
    end
    return m
end
local function G()
    return H()
end
local function F()
    local a = G()
    local n = 0
    while n < 2 do
        n = n + 1
        local b = G()
    end
    return acc
end
print(F())
";
    let ll = compile(source).unwrap_or_else(|(stage, diags)| {
        panic!(
            "the all-faces consumption program must lower clean at {}: {}",
            stage.name(),
            diags_text(&diags)
        )
    });
    assert!(
        !ll.contains("@alm_tbl_get_any"),
        "the Int table's reads stay scalar through every consumed extent"
    );

    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the consumption pin must analyze clean: {:?}",
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
    let k_lit = fn_lit(2);
    let h_lit = fn_lit(3);
    let g_lit = fn_lit(4);
    let f_lit = fn_lit(5);
    let f_call = match &ast[6] {
        Stmt::Print { exprs, .. } => exprs[0].id(),
        _ => panic!("the statement prints one call"),
    };
    let g_a_call = match &fn_body(5)[0] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("F's body opens with the first G call"),
    };
    let g_loop_call = match &fn_body(5)[2] {
        Stmt::While { body, .. } => match &body[1] {
            Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
            _ => panic!("F's loop calls G in a value position"),
        },
        _ => panic!("F's body carries the divergent while"),
    };
    let h_call = match &fn_body(4)[0] {
        Stmt::Return {
            value: Some(call @ Expr::Call { .. }),
            ..
        } => call.id(),
        _ => panic!("G's body is one shared H call"),
    };
    let k_call = match &fn_body(3)[2] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("H's body calls K in a value position"),
    };
    let m_stmt = fn_body(3)[0].id();
    let (t_obj, t_key) = match &fn_body(3)[1] {
        Stmt::Assignment {
            expr: Expr::Index { obj, key, .. },
            ..
        } => (obj.id(), key.id()),
        _ => panic!("H's body assigns m from the t[0] read"),
    };

    assert_eq!(
        shape.instances.len(),
        8,
        "ROOT + F + two G events + two H events (one shared pair) + two \
         K events (one shared pair)"
    );
    let root = &shape.instances[&crate::shape::InstanceId::ROOT];
    let k_f = *root
        .children
        .values()
        .next()
        .unwrap()
        .iter()
        .next()
        .unwrap();
    let f_rec = &shape.instances[&k_f];
    assert_eq!(
        f_rec.call,
        Some(f_call),
        "F's record names its own call expr"
    );
    assert_eq!(
        f_rec.children.len(),
        4,
        "F's extent owns g_a, g_loop, the shared h, and the shared k — \
         four call exprs (the site law: EVERY nested fact lands here)"
    );

    let h_kids = &f_rec.children[&h_call];
    let bases: Vec<_> = h_kids
        .iter()
        .map(|k| shape.instances[k].break_base)
        .collect();
    assert_eq!(
        bases,
        vec![0, 1],
        "the shared H body's two events diverge by F's while — the Break \
         arm must consult the CURRENT event's record"
    );
    let k_kids = &f_rec.children[&k_call];
    let k_bases: Vec<_> = k_kids
        .iter()
        .map(|k| shape.instances[k].break_base)
        .collect();
    assert_eq!(
        k_bases,
        vec![0, 1],
        "K's pair rides the same outer extent and inherits H's events' \
         bases — the depth-three chain mints in body order"
    );

    assert!(
        f_rec.children.contains_key(&h_call),
        "the depth-two H pair's events mint under the outermost instance"
    );
    assert!(
        f_rec.children.contains_key(&k_call),
        "the depth-three K pair's events mint under the outermost \
         instance — never the immediate enclosure"
    );

    assert!(
        matches!(f_rec.index_key_lanes[&t_key], crate::shape::KeyLane::Int),
        "the t[0] key's lane rides the OWNER record (integer keys take \
         the lane lookup)"
    );
    assert_eq!(
        f_rec.access_elems[&t_obj],
        crate::shape::Ty::Int,
        "the t base's elem face rides the OWNER record"
    );
    assert_eq!(
        shape.local_types[&(m_stmt, k_f, "m".to_string())],
        crate::shape::Ty::Int,
        "the bare decl's key instance is the OUTERMOST extent — the \
         exact InstanceId the lowerer derives as \
         chain.first().unwrap_or(ROOT)"
    );
    let g_kids = [&g_a_call, &g_loop_call]
        .into_iter()
        .flat_map(|c| f_rec.children[c].iter().copied())
        .chain(h_kids.iter().copied())
        .chain(k_kids.iter().copied());
    for kid in g_kids {
        let rec = &shape.instances[&kid];
        assert!(
            rec.children.is_empty()
                && rec.index_key_lanes.is_empty()
                && rec.access_elems.is_empty()
                && rec.creation_envs.is_empty()
                && rec.closure_calls.is_empty(),
            "a nested instance's shadow maps are empty by the site law — \
             the facts key against the outermost"
        );
    }

    for (pair_kids, lit) in [(h_kids, h_lit), (k_kids, k_lit)] {
        let fn_nodes: std::collections::BTreeSet<_> = pair_kids
            .iter()
            .map(|k| shape.instances[k].fn_node)
            .collect();
        assert_eq!(
            fn_nodes,
            std::collections::BTreeSet::from([Some(lit)]),
            "every kid of the pair names the SAME callee literal — the \
             pair tier answers identically off any kid"
        );
        let rets: Vec<_> = pair_kids
            .iter()
            .map(|k| shape.instances[k].ret.clone())
            .collect();
        assert!(
            rets.iter().all(|r| r == &rets[0]),
            "every kid of the pair carries the SAME resolved return — \
             the pair tier answers identically off any kid"
        );
    }

    assert_eq!(
        shape.call_fns[&f_call],
        std::collections::BTreeSet::from([f_lit]),
        "call_fns projects the F pair"
    );
    assert_eq!(
        shape.call_fns[&g_a_call] , std::collections::BTreeSet::from([g_lit]),
        "call_fns projects the G_a pair"
    );
    assert_eq!(
        shape.call_fns[&h_call],
        std::collections::BTreeSet::from([h_lit]),
        "call_fns folds the shared H pair's TWO events to one expr entry"
    );
    assert_eq!(
        shape.call_fns[&k_call],
        std::collections::BTreeSet::from([k_lit]),
        "call_fns folds the shared K pair's TWO events to one expr entry"
    );
}
