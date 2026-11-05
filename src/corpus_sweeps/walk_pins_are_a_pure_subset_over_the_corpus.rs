
#[test]
fn walk_pins_are_a_pure_subset_over_the_corpus() {
    let mut bases: Vec<(String, String)> = Vec::new();
    let mut rng = Rng::new(0xA11_5EED);
    for i in 0..16 {
        let (source, _) = gen_program(&mut rng);
        bases.push((format!("generated#{i}"), source));
    }
    bases.extend(corpus_sources());
    assert!(
        bases.len() > 16,
        "the differential leg only found {} base programs — cases/ vanished?",
        bases.len()
    );

    let mut accepted = 0usize;
    for (label, source) in &bases {
        match (compile_with(source, true), compile_with(source, false)) {
            (Ok(on), Ok(off)) => {
                accepted += 1;
                assert_eq!(
                    on, off,
                    "walk pins changed the emitted IR for {label} — the subset discipline broke"
                );
            }
            (Err((stage_on, diags_on)), Err((stage_off, diags_off))) => {
                assert_eq!(
                    stage_on.name(),
                    stage_off.name(),
                    "walk pins changed the rejecting stage for {label}"
                );
                assert_eq!(
                    diags_on, diags_off,
                    "walk pins changed the diagnostics for {label} — the subset discipline broke"
                );
            }
            (on, off) => {
                panic!(
                    "walk pins flipped acceptance for {label}:\n  pins on : {}\n  pins off: {}",
                    match &on {
                        Ok(_) => "accepted".to_string(),
                        Err((s, d)) => format!("{}: {}", s.name(), diags_text(d)),
                    },
                    match &off {
                        Ok(_) => "accepted".to_string(),
                        Err((s, d)) => format!("{}: {}", s.name(), diags_text(d)),
                    }
                );
            }
        }
    }
    assert!(
        accepted >= 10,
        "only {accepted}/{} differential bases reached the lowerer — the accepted leg \
         was never exercised",
        bases.len()
    );
    eprintln!(
        "walk-pin differential: {} bases, {accepted} accepted (byte-identical .ll)",
        bases.len()
    );
}
