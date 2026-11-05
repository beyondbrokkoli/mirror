
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
        local coroutine = require("coroutine")
        local ffi = require("ffi")
        ffi.cdef("void* alm_exec(void*);")
        local lib = ffi.load(arg[0])
        local co = coroutine.create(lib)
        local function Tick() return 1 end
        local pkt = { step = Tick, seed = 3 }
        coroutine.resume(co, pkt)
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
        match compile(source) {
            Err((Stage::Ffi, diags)) => {
                assert!(
                    diags
                        .iter()
                        .any(|d| d.render().contains("ffi surface")
                            && d.render().contains("--exe-only")),
                    "the {label} face's .so-opts rejection lost the gate text: {}",
                    diags_text(&diags)
                );
            }
            other => panic!(
                "the {label} ffi face must reject at the ffi gate under .so opts: {}",
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
