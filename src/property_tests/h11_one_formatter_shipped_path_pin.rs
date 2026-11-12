
#[test]
fn h11_one_formatter_shipped_path_pin() {
    let faces: &[(&str, Stage, &str)] = &[
        (
            "local a = 1\nlocal b =\n",
            Stage::Front,
            "line 2: Syntax Error: Expected expression",
        ),
        (
            "local a = 1\nlocal b = 1 $\n",
            Stage::Front,
            "line 2: Syntax Error: Lexer error",
        ),
        (
            "local t = {[1152921504606846976]=1}\n",
            Stage::Front,
            "line 1: Table Bounds Error: table index overflow",
        ),
        (
            "print(q)\n",
            Stage::Analyze,
            "line 1: Scope Error: reference to undeclared variable 'q'",
        ),
        (
            "local a = \"x\"\nlocal b = a + 1\n",
            Stage::Analyze,
            "line 2: Type Error: '+' requires numeric operands",
        ),
        (
            "local t = {1}\nlocal u = t\nlocal v = t\n",
            Stage::Analyze,
            "line 3: Lifetime Error: 't' may be moved here — reads through a \
             possibly-moved name are rejected at compile time",
        ),
        (
            "local t = {1}\nlocal u = {2}\nt[0] = u\n",
            Stage::Analyze,
            "line 1: Type Error: a table's cells hold one scalar kind — mixing scalar \
             kinds chooses the dynamic cell (Any), but a table value never rides a \
             cell (table site #0)",
        ),
        (
            "local t = {}\nprint(t[0])\n",
            Stage::Analyze,
            "line 1: Type Error: a table is read before it is ever given a value \
             (table site #0)",
        ),
        (
            "local ffi = require(\"ffi\")\nlocal lib = ffi.load(\"libm.so\")\n",
            Stage::Ffi,
            "alm compiler error: ffi.load is --exe-only — a shared object \
             may not load shared objects (the callee seam: a .so exposes \
             alm_arg_kind/alm_exec, it never loads)",
        ),
    ];
    for (source, want_stage, golden) in faces {
        let Err((stage, diags)) = compile(source) else {
            panic!("the face must reject: {golden:?}");
        };
        assert_eq!(
            stage.name(),
            want_stage.name(),
            "the face rejected at the wrong stage: {golden:?}"
        );
        let rendered: Vec<String> = diags.iter().map(|d| d.render()).collect();
        assert_eq!(
            rendered,
            vec![golden.to_string()],
            "the rendered bytes drifted — the format law is byte-stable"
        );
        for d in &diags {
            let _regular = match d.kind {
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
}
