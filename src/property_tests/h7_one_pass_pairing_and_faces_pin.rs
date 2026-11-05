

#[test]
fn h7_one_pass_pairing_and_faces_pin() {
    let source = "\
local t = { 1, { 2 } }
local u = ({ 3 }).x
local v = (function()
    local z = { 4 }
    return z
end).y
if t then
    t[0] = { 5 }
end
";
    let ast = parse_parts(source);
    let ctx = analysis::build_context(&ast);

    let Stmt::LocalDecl { exprs: t_exprs, .. } = &ast[0] else {
        panic!("the first statement declares t");
    };
    let Expr::TableCtor {
        entries: t_entries,
        line: t_line,
        ..
    } = &t_exprs[0]
    else {
        panic!("t's initializer is a ctor");
    };
    let Expr::TableCtor {
        line: nested_line, ..
    } = &t_entries[1].1
    else {
        panic!("t's second entry is the nested ctor");
    };
    let Stmt::LocalDecl { exprs: u_exprs, .. } = &ast[1] else {
        panic!("the second statement declares u");
    };
    let Expr::Field { obj: u_obj, .. } = &u_exprs[0] else {
        panic!("u's initializer reads through a Field obj");
    };
    let Expr::TableCtor { line: u_line, .. } = u_obj.as_ref() else {
        panic!("the Field obj is a ctor — the Field-nested site");
    };
    let Stmt::LocalDecl { exprs: v_exprs, .. } = &ast[2] else {
        panic!("the third statement declares v");
    };
    let Expr::Field { obj: v_obj, .. } = &v_exprs[0] else {
        panic!("v's initializer reads through a Field obj");
    };
    let Expr::Function { body: v_body, .. } = v_obj.as_ref() else {
        panic!("the Field obj is a fn literal — the deepest face");
    };
    let Stmt::LocalDecl {
        exprs: z_exprs,
        loc: z_loc,
        ..
    } = &v_body[0]
    else {
        panic!("the fn body declares z");
    };
    let Expr::TableCtor { line: z_line, .. } = &z_exprs[0] else {
        panic!("z's initializer is a ctor");
    };
    let Stmt::If { then_body, .. } = &ast[3] else {
        panic!("the fourth statement is the if");
    };
    let Stmt::IndexAssign { value, .. } = &then_body[0] else {
        panic!("the if arm stores into t");
    };
    let Expr::TableCtor { line: s_line, .. } = value else {
        panic!("the stored value is a ctor");
    };

    assert_eq!(
        (*t_line, *nested_line, *u_line, *z_line, *s_line),
        (1, 1, 2, 4, 8)
    );
    let ctor_at = |e: &Expr| ctx.sites.get(&e.id()).copied();
    let pairs = [
        (&t_exprs[0] as &Expr, 0usize, *t_line as usize),
        (&t_entries[1].1, 1, *nested_line as usize),
        (u_obj.as_ref(), 2, *u_line as usize),
        (&z_exprs[0], 3, *z_line as usize),
        (value, 4, *s_line as usize),
    ];
    for (what, (ctor, want_id, want_line)) in [
        ("t's ctor", pairs[0]),
        ("t's nested ctor", pairs[1]),
        ("the Field-nested ctor", pairs[2]),
        ("the fn body's ctor", pairs[3]),
        ("the if arm's ctor", pairs[4]),
    ] {
        let Some(id) = ctor_at(ctor) else {
            panic!("the one pass numbered every ctor — {what} has no site id");
        };
        assert_eq!(id, want_id, "{what} takes pre-order site {want_id}");
        assert_eq!(
            ctx.ctor_lines[id], want_line,
            "{what}: ctor_lines[id] is its own '{{' line — one arm, two mints"
        );
    }
    assert_eq!(ctx.sites.len(), ctx.ctor_lines.len());
    assert_eq!(z_loc.line, 4);
    assert_eq!(
        Some(v_body[0].loc().line as usize),
        Some(4),
        "a fn body under a Field obj is stamped like any other"
    );
    assert!(ctx.fn_touched.contains_key(&v_obj.as_ref().id()));
}
