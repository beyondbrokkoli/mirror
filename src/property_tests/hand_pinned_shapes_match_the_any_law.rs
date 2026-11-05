
#[test]
fn hand_pinned_shapes_match_the_any_law() {
    let cases: &[(&str, Elem)] = &[
        (
            "local t = {1}\nt = nil\nt = {}\nt[0] = 2.5\nprint(t[0])\n",
            Elem::Any,
        ),
        ("local t = {1}\nt[0] = 2.5\nprint(t[0])\n", Elem::Any),
        (
            "local t = {1}\nt = {}\nt[0] = 2.5\nprint(t[0])\n",
            Elem::Any,
        ),
        ("local t = {}\nt[0] = 2.5\nprint(t[0])\n", Elem::Flt),
        (
            "local t = {}\nt[0] = 1\nt[1] = 2.5\nprint(t[0])\n",
            Elem::Any,
        ),
        (
            "local t = {1}\nif true then\n  t = nil\n  t = {2.5}\nend\nprint(t[0])\n",
            Elem::Any,
        ),
        (
            "local t = {1}\nprint(t[0])\nt[0] = 2.5\nprint(t[0])\n",
            Elem::Any,
        ),
        ("local t = {1}\nt[0] = 2\nt = {3}\nprint(t[0])\n", Elem::Int),
        ("local t = {true}\nt[1] = false\nprint(t[1])\n", Elem::Bool),
        ("local t = {\"s\"}\nt[0] = 2.5\nprint(t[0])\n", Elem::Any),
        (
            "local t = {1}\nif false then\n  t[0] = 2.5\nend\nprint(t[0])\n",
            Elem::Any,
        ),
        (
            "local t = {}\nfor i = 0, 3 do\n  t[i] = 2.5\nend\nprint(t[0])\n",
            Elem::Flt,
        ),
        (
            "local t = {1}\nfor i = 0, 3 do\n  t[i] = 2.5\nend\nprint(t[0])\n",
            Elem::Any,
        ),
        (
            "local t = {true}\nwhile true do\n  t[0] = false\n  break\nend\nprint(t[0])\n",
            Elem::Bool,
        ),
    ];
    for (source, expected) in cases {
        match compile(source) {
            Ok(ll) => assert_elem_congruence(&ll, *expected, "hand-pinned shape"),
            Err((stage, diags)) => panic!(
                "{}",
                report(
                    source,
                    *expected,
                    format!(
                        "pinned shape was rejected at {}: {}",
                        stage.name(),
                        diags_text(&diags)
                    ),
                )
            ),
        }
    }
}
