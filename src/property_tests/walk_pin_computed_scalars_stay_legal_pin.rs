#[test]
fn walk_pin_computed_scalars_stay_legal_pin() {
    let source = "local n = arg[0] + 1\n\
                  local m = n + 1\n\
                  local copy = m\n\
                  local t = { x = arg[0] + 1 }\n\
                  print(m + copy + t.x)\n";
    match (compile_with(source, true), compile_with(source, false)) {
        (Ok(on_ir), Ok(off_ir)) => {
            assert_eq!(
                on_ir, off_ir,
                "the computed-scalar pins changed the emitted IR — the subset discipline broke"
            );
        }
        (on, off) => panic!(
            "the computed-scalar program must stay legal on both lever positions:\n  \
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
