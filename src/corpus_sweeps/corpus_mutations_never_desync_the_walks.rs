
#[test]
fn corpus_mutations_never_desync_the_walks() {
    const PER_OP: usize = 3;
    let mut checked = 0usize;
    let mut analyzer_accepted = 0usize;

    for (name, original) in &corpus_sources() {
        if compile(original).is_err() {
            continue;
        }
        let lines: Vec<&str> = original.lines().collect();
        let eligible: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                let t = l.trim();
                !t.is_empty() && !t.starts_with("--")
            })
            .map(|(i, _)| i)
            .collect();
        if eligible.is_empty() {
            continue;
        }
        let mut rng = Rng::new(fnv1a(name.as_bytes()));
        for op in 0..3 {
            for _ in 0..PER_OP {
                let i = eligible[rng.below(eligible.len())];
                let mutant: Vec<&str> = match op {
                    0 => {
                        let mut m = lines.clone();
                        m.remove(i.min(m.len() - 1));
                        m
                    }
                    1 => {
                        let mut m = lines.clone();
                        let at = i.min(m.len());
                        m.insert(at, lines[i]);
                        m
                    }
                    _ => {
                        let mut m = lines.clone();
                        if i + 1 < m.len() {
                            m.swap(i, i + 1);
                        }
                        m
                    }
                };
                let mutant = mutant.join("\n");
                match compile(&mutant) {
                    Ok(_) => {
                        analyzer_accepted += 1;
                    }
                    Err((stage, diags))
                        if matches!(stage, Stage::Panicked | Stage::Lower | Stage::Backend) =>
                    {
                        let crime = match stage {
                            Stage::Panicked => "the compiler crashed",
                            _ => "checker-accepted mutant failed to lower or render \
                                  clean",
                        };
                        panic!(
                            "{crime} — mutant of {name} failed at {}\n\
                             mutation: op {op} at line {}\ndiagnostics: \
                             {}\nmutant:\n{}",
                            stage.name(),
                            i + 1,
                            diags_text(&diags),
                            indent(&mutant),
                        );
                    }
                    Err(_) => {
                    }
                }
                checked += 1;
            }
        }
    }
    assert!(
        checked >= 22,
        "corpus mutation leg only exercised {checked} mutants — \
         the corpus shrank below one round"
    );
    assert!(
        analyzer_accepted >= 1,
        "0/{checked} corpus mutants survived Stage::Analyze — the lowerer/backend \
         implication was never actually exercised"
    );
    eprintln!("corpus mutation coverage: checked={checked} analyzer_accepted={analyzer_accepted}");
}
