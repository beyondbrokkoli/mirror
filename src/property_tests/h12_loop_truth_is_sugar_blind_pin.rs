#[test]
fn h12_loop_truth_is_sugar_blind_pin() {
    let source = "\
local function Make(scale)
    local function Poll(x)
        return x + scale
    end
    return { Poll = Poll }
end
local d = Make(1)
local t = { [0] = 0 }
local function Core(n)
    local i = 0
    while true do
        if i == n then
            break
        end
        i = d.Poll(i)
    end
    t[0] = t[0] + i
    return i
end
local function Host(k)
    return Core(k)
end
local function F()
    local a = Host(2)
    for j = 0, 1 do
        a = Host(3)
    end
    return a
end
print(F())
";
    let ll = compile(source).unwrap_or_else(|(stage, diags)| {
        panic!(
            "the sugar-blind loop-truth program must lower clean at {}: {}",
            stage.name(),
            diags_text(&diags)
        )
    });
    assert!(
        !ll.contains("@alm_tbl_get_any"),
        "the Int table and the closure record stay scalar through every \
         consumed extent"
    );

    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the sugar-blind pin must analyze clean: {:?}",
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
    let make_lit = fn_lit(0);
    let core_lit = fn_lit(3);
    let host_lit = fn_lit(4);
    let make_call = match &ast[1] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("d binds the Make call"),
    };
    let f_call = match &ast[6] {
        Stmt::Print { exprs, .. } => exprs[0].id(),
        _ => panic!("the statement prints one call"),
    };

    let f_body = fn_body(5);
    let host_a = match &f_body[0] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("F's body opens with the flat Host call"),
    };
    let host_b = match &f_body[1] {
        Stmt::Do { body, .. } => {
            assert!(
                matches!(
                    body.get(1),
                    Some(Stmt::LocalDecl { names, .. }) if names.len() == 1 && names[0] == "for$limit"
                ),
                "the for-desugar fingerprint: counter, for$limit latch, while"
            );
            match &body[2] {
                Stmt::While { body: wbody, .. } => match &wbody[0] {
                    Stmt::Do { body: inner, .. } => match &inner[0] {
                        Stmt::Assignment { expr, .. } => expr.id(),
                        _ => panic!("the desugared body assigns the Host call"),
                    },
                    _ => panic!("the while wraps the user body in its own Do"),
                },
                _ => panic!("the desugar's third statement is the While"),
            }
        }
        _ => panic!("F's second statement is the desugared for"),
    };
    let core_call = match &fn_body(4)[0] {
        Stmt::Return {
            value: Some(call @ Expr::Call { .. }),
            ..
        } => call.id(),
        _ => panic!("Host's body is one shared Core call"),
    };
    let poll_call = match &fn_body(3)[1] {
        Stmt::While { body, .. } => match &body[1] {
            Stmt::Assignment {
                expr: call @ Expr::Call { .. },
                ..
            } => call.id(),
            _ => panic!("Core's loop body carries the dispatch in a value position"),
        },
        _ => panic!("Core's body carries its own while"),
    };
    let poll_value = match &fn_body(0)[1] {
        Stmt::Return {
            value: Some(Expr::TableCtor { entries, .. }),
            ..
        } => entries[0].1.id(),
        _ => panic!("Make returns the closure record"),
    };

    assert_eq!(
        shape.instances.len(),
        9,
        "ROOT + Make + F + two Host events + two Core events (one shared \
         pair) + two Poll events (one shared pair)"
    );

    let root = &shape.instances[&crate::shape::InstanceId::ROOT];
    let m_kid = *root.children[&make_call].iter().next().unwrap();
    let f_kid = *root.children[&f_call].iter().next().unwrap();
    let f_rec = &shape.instances[&f_kid];

    let m_rec = &shape.instances[&m_kid];
    assert_eq!(
        m_rec.creation_envs.len(),
        1,
        "one creation site inside Make's extent"
    );
    assert!(
        m_rec.creation_envs.contains_key(&poll_value),
        "the Poll value read is Make's own extent's creation fact"
    );
    assert!(
        m_rec.children.is_empty(),
        "Make's body makes no calls — no children under its extent"
    );

    let host_kids = [&host_a, &host_b].map(|c| *f_rec.children[c].iter().next().unwrap());
    assert_eq!(
        shape.instances[&host_kids[0]].break_base,
        0,
        "the flat Host event mints at base 0"
    );
    assert_eq!(
        shape.instances[&host_kids[1]].break_base,
        1,
        "the Host event under the DESUGARED for's while mints at base 1 — \
         loop truth is sugar-blind: the mint arithmetic counts the \
         sugar's while like any written one"
    );

    let core_kids = &f_rec.children[&core_call];
    assert_eq!(
        core_kids.len(),
        2,
        "Core's single call expr in Host's body mints once per Host event \
         — both under F, the outermost"
    );
    let core_bases: Vec<_> = core_kids
        .iter()
        .map(|k| shape.instances[k].break_base)
        .collect();
    assert_eq!(
        core_bases, vec![0, 1],
        "Core's events inherit their events' bases — the Break arm must \
         consult the CURRENT event's base; an any-kid pick that lands \
         base 1 on the depth-0 event fires the crossing guard on this \
         legal program"
    );

    assert!(
        f_rec.closure_calls.contains(&poll_call),
        "the dispatch is the OWNER's fact — closure_calls on the \
         outermost record, read there by the lowerer at depth three"
    );
    let poll_kids = &f_rec.children[&poll_call];
    assert_eq!(
        poll_kids.len(),
        2,
        "the closure's events mint under the SAME outermost extent as \
         every other nested call — the site law holds for dispatches"
    );
    let poll_bases: Vec<_> = poll_kids
        .iter()
        .map(|k| shape.instances[k].break_base)
        .collect();
    assert_eq!(
        poll_bases, vec![1, 2],
        "the dispatch's base COMPOUNDS: each Poll event's base is its \
         Core event's base PLUS the dispatch's own loop depth inside the \
         body (the dispatch sits inside Core's while) — the arithmetic \
         threads sugar-fed event bases and body-relative depth into one \
         absolute number"
    );
    for k in poll_kids.iter().copied() {
        let rec = &shape.instances[&k];
        assert_eq!(
            rec.fn_node,
            Some(match &ast[0] {
                Stmt::LocalDecl { exprs, .. } => match &exprs[0] {
                    Expr::Function { body, .. } => match &body[0] {
                        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
                        _ => panic!("Make's body opens with the Poll local"),
                    },
                    _ => panic!("Make binds a fn literal"),
                },
                _ => panic!("the statement declares a fn"),
            }),
            "every Poll event names the Poll literal — the pair tier"
        );
        assert_eq!(
            rec.env_face,
            Some(std::collections::BTreeMap::from([(
                "scale".to_string(),
                crate::shape::Ty::Int
            )])),
            "a dispatch's env face rides its events (the captured scalar's box)"
        );
        assert_eq!(rec.ret, Some(crate::shape::Ty::Int), "Poll's resolved return");
    }

    for kid in host_kids
        .into_iter()
        .chain(core_kids.iter().copied())
        .chain(poll_kids.iter().copied())
    {
        let rec = &shape.instances[&kid];
        assert!(
            rec.children.is_empty()
                && rec.index_key_lanes.is_empty()
                && rec.access_elems.is_empty()
                && rec.creation_envs.is_empty()
                && rec.closure_calls.is_empty(),
            "a nested instance's maps are empty by the site law — the \
             closure dispatch's facts key against the outermost, like \
             every other extent fact"
        );
    }

    assert_eq!(
        shape.call_fns[&core_call],
        std::collections::BTreeSet::from([core_lit]),
        "call_fns folds Core's two events to one expr entry"
    );
    assert_eq!(
        shape.call_fns[&host_b],
        std::collections::BTreeSet::from([host_lit]),
        "call_fns projects the sugar-hosted Host pair"
    );
    let _ = make_lit;
}
