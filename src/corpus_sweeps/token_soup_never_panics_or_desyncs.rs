
fn compile_tokenized(
    source: &str,
    tokens: Vec<(lexer::Token<'_>, usize)>,
) -> Result<String, (Stage, Vec<Diagnostic>)> {
    compile_tokenized_with(source, tokens, true)
}

#[test]
fn token_soup_never_panics_or_desyncs() {
    let mut bases: Vec<(String, String)> = Vec::new();
    let mut rng = Rng::new(0x5050_5EED);
    for i in 0..16 {
        let (source, _) = gen_program(&mut rng);
        bases.push((format!("generated#{i}"), source));
    }
    for (name, source) in corpus_sources() {
        if compile(&source).is_ok() {
            bases.push((name, source));
        }
    }
    assert!(
        bases.len() >= 16,
        "token soup only found {} base programs — cases/ vanished?",
        bases.len()
    );

    const MUTANTS_PER_BASE: usize = 20;
    let mut exercised = 0usize;
    let mut accepted = 0usize;
    for (label, source) in &bases {
        let mut toks: Vec<(lexer::Token<'_>, usize)> = Vec::new();
        let mut lex = lexer::Token::lexer(source);
        while let Some(Ok(t)) = lex.next() {
            toks.push((t, lex.span().start));
        }
        if toks.is_empty() {
            continue;
        }
        let mut rng = Rng::new(fnv1a(label.as_bytes()));
        for _ in 0..MUTANTS_PER_BASE {
            let edits = 1 + rng.below(4);
            let mut soup = toks.clone();
            for _ in 0..edits {
                if soup.is_empty() {
                    break;
                }
                let at = rng.below(soup.len());
                match rng.below(3) {
                    0 => {
                        soup.remove(at);
                    }
                    1 => {
                        let run = 1 + rng.below(3.min(soup.len() - at));
                        let dup: Vec<(lexer::Token<'_>, usize)> = soup[at..at + run].to_vec();
                        for (k, t) in dup.into_iter().enumerate() {
                            soup.insert(at + k, t);
                        }
                    }
                    _ => {
                        if at + 1 < soup.len() {
                            soup.swap(at, at + 1);
                        }
                    }
                }
            }
            if soup.is_empty() {
                continue;
            }
            exercised += 1;
            match compile_tokenized(source, soup.clone()) {
                Ok(_) => accepted += 1,
                Err((stage, diags))
                    if matches!(stage, Stage::Panicked | Stage::Lower | Stage::Backend) =>
                {
                    let crime = match stage {
                        Stage::Panicked => "the compiler crashed on token soup",
                        _ => "checker-accepted token soup failed to lower or render \
                              clean",
                    };
                    panic!(
                        "{crime} — base {label} failed at {}\ndiagnostics: {}\nsoup \
                         (Debug tokens):\n{}\nbase:\n{}",
                        stage.name(),
                        diags_text(&diags),
                        soup.iter()
                            .map(|(t, _)| format!("  {t:?}"))
                            .collect::<Vec<_>>()
                            .join("\n"),
                        indent(source),
                    );
                }
                Err(_) => {
                }
            }
        }
    }
    assert!(
        exercised >= 400,
        "token soup only exercised {exercised} mutants — the base pool dried up"
    );
    assert!(
        accepted >= 10,
        "0/{exercised} soup mutants compiled — the accepted leg of the implication \
         was never exercised"
    );
    eprintln!(
        "token soup coverage: bases={} exercised={exercised} accepted={accepted}",
        bases.len()
    );
}
