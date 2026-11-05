#[test]
fn h12_creation_envs_site_law_at_depth_two_pin() {
    let source = "\
local function G()
    local function Inner(x) return x end
    return Inner
end
local function F()
    local q
    local f = G()
    return 0
end
F()
F()
print(0)
";
    let ll = compile(source).unwrap_or_else(|(stage, diags)| {
        panic!(
            "the depth-two creation program must lower clean at {}: {}",
            stage.name(),
            diags_text(&diags)
        )
    });
    assert!(
        ll.contains("@alm_print_int"),
        "the scalar print rides the boundary"
    );

    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);
    let shape = crate::shape::analyze_with(&ctx, true);
    assert!(
        shape.diagnostics.is_empty(),
        "the creation pin must analyze clean: {:?}",
        crate::property_tests::diags_text(&shape.diagnostics)
    );
    assert_eq!(
        shape.instances.len(),
        5,
        "ROOT + two F events + two G events — creations mint no instance"
    );

    let g_body = match &ast[0] {
        Stmt::LocalDecl { exprs, .. } => match &exprs[0] {
            Expr::Function { body, .. } => body,
            _ => panic!("the local binds G's fn literal"),
        },
        _ => panic!("the statement declares G"),
    };
    let inner_lit = match &g_body[0] {
        Stmt::LocalDecl { exprs, .. } => exprs[0].id(),
        _ => panic!("G's body declares Inner"),
    };
    let inner_read = match &g_body[1] {
        Stmt::Return {
            value: Some(e), ..
        } => e.id(),
        _ => panic!("G's body returns the value read"),
    };

    let root = &shape.instances[&crate::shape::InstanceId::ROOT];
    assert_eq!(
        root.children.len(),
        2,
        "the two top-level F calls self-pair to ROOT"
    );
    assert!(
        root.creation_envs.is_empty(),
        "a callee read never mints — creation is the value read's land"
    );

    let mut saw_creation = 0;
    for set in root.children.values() {
        let f_inst = *set.iter().next().unwrap();
        let f_rec = &shape.instances[&f_inst];
        assert_eq!(
            f_rec.children.len(),
            1,
            "F's extent minted G"
        );
        assert_eq!(
            f_rec.creation_envs.get(&inner_read),
            Some(&inner_lit),
            "the value read inside G's body mints against F's call — the \
             OUTERMOST extent, per site"
        );
        saw_creation += 1;
        let g_inst = *f_rec.children.values().next().unwrap().iter().next().unwrap();
        let g_rec = &shape.instances[&g_inst];
        assert!(
            g_rec.creation_envs.is_empty() && g_rec.children.is_empty(),
            "G's own record carries no shadow maps — the site law keys \
             against the OUTERMOST extent"
        );
        assert!(
            !f_rec.children.is_empty(),
            "the creation pair's events mint under F's record"
        );
    }
    assert_eq!(saw_creation, 2, "one creation entry per site");
}
