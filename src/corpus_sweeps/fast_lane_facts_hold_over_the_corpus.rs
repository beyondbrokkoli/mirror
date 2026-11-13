
#[test]
fn fast_lane_facts_hold_over_the_corpus() {
    fn analyze(source: &str) -> Option<crate::shape::ShapeFacts> {
        // The corpus carries negative cases the front end rejects before
        // analysis (embedded NUL literals, now a lexer Err — the detailed
        // diagnostic, no longer a panic); the sweeps treat them as
        // rejections, so this pin does too.
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut tokens = Vec::new();
            let mut offsets = Vec::new();
            let mut lexer = lexer::Token::lexer(source);
            loop {
                match lexer.next() {
                    Some(Ok(token)) => {
                        offsets.push(lexer.span().start);
                        tokens.push(token);
                    }
                    Some(Err(_)) => return None,
                    None => break,
                }
            }
            let mut parser =
                crate::parser::Parser::new(tokens.into_iter().zip(offsets).collect(), source);
            let ast = parser.parse_program();
            if !parser.diagnostics.is_empty() {
                return None;
            }
            let ctx = crate::analysis::build_context(&ast);
            let facts = crate::shape::analyze_with(&ctx, true);
            facts.diagnostics.is_empty().then_some(facts)
        }));
        caught.ok().flatten()
    }

    // NodeIds are minted in parse order, so the fact maps iterate in
    // source order and the projection is comparable across two runs.
    fn lane_shape(f: &crate::shape::ShapeFacts) -> (Vec<(String, bool)>, Vec<(Vec<String>, Vec<String>, usize)>) {
        let stores = f
            .fast_stores
            .values()
            .map(|e| (e.table.clone(), e.row_reserve))
            .collect();
        let plans = f
            .reserve_plans
            .values()
            .map(|p| (p.pre_header.clone(), p.nested.clone(), p.ctor_reserves.len()))
            .collect();
        (stores, plans)
    }

    let mut analyzed = 0usize;
    let mut total_stores = 0usize;
    let mut total_plans = 0usize;
    for (name, source) in corpus_sources() {
        let Some(facts) = analyze(&source) else {
            continue;
        };
        analyzed += 1;
        for elig in facts.fast_stores.values() {
            assert!(
                facts.reserve_plans.contains_key(&elig.loop_id),
                "{name}: a fast store's loop carries no reserve plan"
            );
        }
        let again = analyze(&source)
            .unwrap_or_else(|| panic!("{name}: the second analysis run rejected the source"));
        assert_eq!(
            lane_shape(&facts),
            lane_shape(&again),
            "{name}: the fast-lane facts are not a pure function of the source"
        );
        total_stores += facts.fast_stores.len();
        total_plans += facts.reserve_plans.len();
    }
    assert!(analyzed > 100, "only {analyzed} corpus cases reached analysis");
    assert!(
        total_stores >= 20 && total_plans >= 10,
        "the corpus fast lane collapsed: {total_stores} stores over {total_plans} plans"
    );
    eprintln!(
        "fast-lane corpus pin: {analyzed} analyzed, {total_stores} stores, {total_plans} plans"
    );
}
