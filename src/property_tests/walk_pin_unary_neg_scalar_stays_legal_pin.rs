#[test]
fn walk_pin_unary_neg_scalar_stays_legal_pin() {
    let source = "local k = -arg[0]\nprint(k)\n";
    match (compile_with(source, true), compile_with(source, false)) {
        (Ok(on_ir), Ok(off_ir)) => {
            assert_eq!(
                on_ir, off_ir,
                "the unary-neg pin changed the emitted IR — the subset discipline broke"
            );
        }
        (on, off) => panic!(
            "the unary-neg program must stay legal on both lever positions:\n  \
             pins on : {}\n  pins off: {}",
            match &on {
                Ok(_) => "accepted".to_string(),
                Err((s, d)) => format!("{}: {}", s.name(), diags_text(d)),
            },
            match &off {
                Ok(_) => "accepted".to_string(),
                Err((s, d)) => format!("{}: {}", s.name(), diags_text(d)),
            }
        ),
    }
}
