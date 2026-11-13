
#[test]
fn w3a_crossing_faces_reject_on_both_drive_and_spawn() {
    let drive_face = r#"
        local ffi = require("ffi")
        ffi.cdef("void* alm_exec(void*);")
        local lib = ffi.load(arg[0])
        local function Tick() return 1 end
        local pkt = { step = Tick, seed = 3 }
        lib.alm_exec(pkt)
    "#;
    let spawn_face = r#"
        local async = require("async")
        local ffi = require("ffi")
        ffi.cdef("void* alm_exec(void*);")
        local lib = ffi.load(arg[0])
        local co = async.create(lib)
        local function Tick() return 1 end
        local pkt = { step = Tick, seed = 3 }
        async.resume(co, pkt)
    "#;
    for (label, source, needle) in [
        (
            "drive",
            drive_face,
            "a closure cannot cross a module boundary — the drive table marshals flat \
             cells only",
        ),
        (
            "spawn",
            spawn_face,
            "a closure cannot cross a module boundary — the spawn table marshals flat \
             cells only",
        ),
    ] {
        // Under .so opts the face rejects one of two ways (M4 order):
        // the analyzer runs before the load gate, so the crossing law
        // may fire first at Stage::Analyze; otherwise the ffi.load in
        // the face trips the gate at Stage::Ffi. Either law rejecting
        // is the face refusing to cross under .so.
        match compile(source) {
            Err((Stage::Ffi, diags)) => {
                assert!(
                    diags
                        .iter()
                        .any(|d| d.render().contains("ffi.load")
                            && d.render().contains("--exe-only")),
                    "the {label} face's .so-opts rejection lost the gate text: {}",
                    diags_text(&diags)
                );
            }
            Err((Stage::Analyze, diags)) => {
                assert!(
                    diags.iter().any(|d| d.render().contains(needle)),
                    "the {label} face rejected at analysis with the wrong text: {}",
                    diags_text(&diags)
                );
            }
            other => panic!(
                "the {label} ffi face must reject under .so opts: {}",
                match other {
                    Ok(_) => "accepted".to_string(),
                    Err((s, d)) => format!("{}: {}", s.name(), diags_text(&d)),
                }
            ),
        }
        match compile_exe(source) {
            Err((Stage::Analyze, diags)) => {
                assert!(
                    diags.iter().any(|d| d.render().contains(needle)),
                    "the {label} face rejected with the wrong text: {}",
                    diags_text(&diags)
                );
            }
            other => panic!(
                "the {label} crossing face did not reject at analysis under exe opts: {}",
                match other {
                    Ok(_) => "accepted".to_string(),
                    Err((s, d)) => format!("{}: {}", s.name(), diags_text(&d)),
                }
            ),
        }
    }
}
