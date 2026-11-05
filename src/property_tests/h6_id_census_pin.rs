
#[test]
#[should_panic(expected = "identity census")]
fn h6_id_census_pin() {
    use crate::ast::Loc;
    let at = Loc { line: 1, col: 1 };
    let orig = Stmt::Break {
        loc: at,
        id: NodeId::mint(),
    };
    let ast = vec![
        Stmt::Break {
            loc: at,
            id: NodeId::mint(),
        },
        orig.clone(),
        orig,
    ];
    analysis::build_context(&ast);
}
