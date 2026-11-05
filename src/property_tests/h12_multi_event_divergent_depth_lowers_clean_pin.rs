#[test]
fn h12_multi_event_divergent_depth_lowers_clean_pin() {
    let source = "\
local t = { [0] = 7 }
local acc = 0
local function H()
    local m
    acc = acc + 1
    return t[0]
end
local function G()
    return H()
end
local function F()
    local q
    local a = G()
    local n = 0
    while n < 9 do
        n = n + 1
        local c = G()
    end
    return a
end
local z
F()
";
    let ll = compile(source).unwrap_or_else(|(stage, diags)| {
        panic!(
            "the divergent-depth multi-event program must lower clean at {}: {}",
            stage.name(),
            diags_text(&diags)
        )
    });
    assert!(
        !ll.contains("@alm_tbl_get_any"),
        "the Int table's reads stay scalar through the divergent extents"
    );
}
