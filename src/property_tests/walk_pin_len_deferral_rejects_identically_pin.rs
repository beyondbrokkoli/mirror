#[test]
fn walk_pin_len_deferral_rejects_identically_pin() {
    let source = "print(#arg[0])\nlocal n = arg[0] + 1\n";
    match (compile_with(source, true), compile_with(source, false)) {
        (Err((stage_on, diags_on)), Err((stage_off, diags_off))) => {
            assert_eq!(
                stage_on.name(),
                "shape analyzer + checker (one walk)",
                "the incident case must reject at the analyzer, not panic"
            );
            assert_eq!(stage_on.name(), stage_off.name());
            assert_eq!(diags_on, diags_off);
            assert!(
                diags_text(&diags_on)
                    .contains("'#' requires a Table, String, or Any operand, got Integer"),
                "the len-deferral diagnostic changed: {}",
                diags_text(&diags_on)
            );
        }
        (on, off) => panic!(
            "the len-deferral incident case must reject on both lever positions:\n  \
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
