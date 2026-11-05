
#[test]
fn h11_kinds_and_desync_render_law_pin() {
    let table: &[(Diagnostic, &str)] = &[
        (Diagnostic::syntax("body"), "line 1: Syntax Error: body"),
        (Diagnostic::ty("body"), "line 1: Type Error: body"),
        (Diagnostic::scope("body"), "line 1: Scope Error: body"),
        (Diagnostic::lifetime("body"), "line 1: Lifetime Error: body"),
        (Diagnostic::lower("body"), "line 1: Lower Error: body"),
        (
            Diagnostic::bounds("body"),
            "line 1: Table Bounds Error: body",
        ),
    ];
    for (d, golden) in table {
        let stamped = d.clone().with_site(Some(1));
        assert_eq!(stamped.render(), *golden, "the prefix table is exact");
        assert!(d.site.is_none(), "the mint carries no site");
    }
    assert_eq!(Diagnostic::bare("refusal text").render(), "refusal text");
    for d in [
        Diagnostic::bare("x"),
        Diagnostic::syntax("x"),
        Diagnostic::ty("x"),
        Diagnostic::scope("x"),
        Diagnostic::lifetime("x"),
        Diagnostic::lower("x"),
        Diagnostic::bounds("x"),
    ] {
        let _face = match d.kind {
            DiagKind::Bare => 0,
            DiagKind::Syntax => 1,
            DiagKind::Type => 2,
            DiagKind::Scope => 3,
            DiagKind::Lifetime => 4,
            DiagKind::Lower => 5,
            DiagKind::Bounds => 6,
        };
    }
}
