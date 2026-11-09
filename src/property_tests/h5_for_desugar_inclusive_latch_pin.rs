
#[test]
fn h5_for_desugar_inclusive_latch_pin() {
    // The inclusive surface's expansion golden: identical statement
    // shape, identical guards, identical stamps — only the latch
    // initializer reads `limit + 1`. The census arm needs no change;
    // this pin is the per-surface twin of
    // h5_for_desugar_reports_at_the_header_pin.
    let source = "for i = 0, 4 do\n    print(i)\nend\n";
    let ast = parse_parts_with(source, true);
    assert_eq!(ast.len(), 1);
    let Stmt::Do { body, .. } = &ast[0] else {
        panic!("the for-desugar rides a Do block");
    };
    // The shape is the exclusive desugar's shape, statement for statement.
    assert!(matches!(&body[0], Stmt::LocalDecl { names, .. } if names[0] == "i"));
    let Stmt::LocalDecl {
        names,
        exprs,
        loc,
        ..
    } = &body[1]
    else {
        panic!("the latch decl is a LocalDecl");
    };
    assert_eq!(names, &vec!["for$limit".to_string()]);
    // The one visible difference: the latch initializer is `limit + 1`
    // — the user limit on the left, the literal 1 on the right.
    assert_eq!(exprs.len(), 1);
    let Expr::BinaryOp {
        op: BinOp::Add,
        left,
        right,
        ..
    } = &exprs[0]
    else {
        panic!("the inclusive latch initializer is `limit + 1`");
    };
    assert!(matches!(left.as_ref(), Expr::Integer { value: 4, .. }));
    assert!(matches!(right.as_ref(), Expr::Integer { value: 1, .. }));
    assert_eq!(*loc, crate::ast::Loc { line: 1, col: 1 });
    let Stmt::While {
        condition,
        body: wbody,
        ..
    } = &body[2]
    else {
        panic!("the block's third statement is the While");
    };
    // The guard never changes: `i < for$limit` in both modes.
    assert!(matches!(
        condition,
        Expr::BinaryOp {
            op: BinOp::LessThan,
            ..
        }
    ));
    let line_of = |s: &Stmt| Some(s.loc().line as usize);
    for (what, node) in [
        ("block Do", &ast[0] as &Stmt),
        ("counter decl", &body[0]),
        ("for$limit latch decl", &body[1]),
        ("While", &body[2]),
        ("body's inner Do", &wbody[0]),
        ("synthetic increment", &wbody[1]),
    ] {
        assert_eq!(
            line_of(node),
            Some(1),
            "the {what} must report the for header's line"
        );
    }
}
