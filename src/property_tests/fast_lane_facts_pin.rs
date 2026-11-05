
fn lane_facts(source: &str) -> Result<(Vec<Stmt>, crate::shape::ShapeFacts), String> {
    let mut tokens = Vec::new();
    let mut offsets = Vec::new();
    let mut lexer = lexer::Token::lexer(source);
    loop {
        match lexer.next() {
            Some(Ok(token)) => {
                offsets.push(lexer.span().start);
                tokens.push(token);
            }
            Some(Err(_)) => return Err("lexer error".to_string()),
            None => break,
        }
    }
    let mut parser = Parser::new(tokens.into_iter().zip(offsets).collect(), source);
    let ast = parser.parse_program();
    if !parser.diagnostics.is_empty() {
        return Err(diags_text(&parser.diagnostics));
    }
    let ctx = analysis::build_context(&ast);
    let facts = crate::shape::analyze_with(&ctx, true);
    if !facts.diagnostics.is_empty() {
        return Err(diags_text(&facts.diagnostics));
    }
    Ok((ast, facts))
}

/// The corpus statement order is the NodeId mint order, so an
/// IndexAssign's position in this list is its position in the source.
fn store_sites(stmts: &[Stmt], out: &mut Vec<(NodeId, String)>) {
    fn base_of(e: &Expr) -> String {
        match e {
            Expr::Identifier { name, .. } => name.clone(),
            Expr::Index { obj, .. } => base_of(obj),
            _ => "<expr>".to_string(),
        }
    }
    fn walk_expr(e: &Expr, out: &mut Vec<(NodeId, String)>) {
        match e {
            Expr::Function { body, .. } => store_sites(body, out),
            Expr::TableCtor { entries, .. } => {
                for (k, v) in entries {
                    if let crate::ast::CtorKey::Expr(ke) = k {
                        walk_expr(ke, out);
                    }
                    walk_expr(v, out);
                }
            }
            _ => {}
        }
    }
    for s in stmts {
        match s {
            Stmt::IndexAssign { obj, .. } => {
                out.push((s.id(), base_of(obj)));
            }
            Stmt::While { condition, body, .. } => {
                walk_expr(condition, out);
                store_sites(body, out);
            }
            Stmt::Do { body, .. } => store_sites(body, out),
            Stmt::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                walk_expr(condition, out);
                store_sites(then_body, out);
                store_sites(else_body, out);
            }
            Stmt::LocalDecl { exprs, .. } | Stmt::Print { exprs, .. } => {
                for e in exprs {
                    walk_expr(e, out);
                }
            }
            Stmt::Assignment { expr, .. } | Stmt::Expr { expr, .. } => walk_expr(expr, out),
            Stmt::Return { value: Some(e), .. } => walk_expr(e, out),
            _ => {}
        }
    }
}

fn expect_lane(
    source: &str,
    want_stores: &[(&str, bool)],
    want_plans: &[(&[&str], &[&str])],
) {
    let (ast, facts) = lane_facts(source).unwrap_or_else(|e| panic!("lane pin source rejected: {e}"));
    let mut sites = Vec::new();
    store_sites(&ast, &mut sites);
    let mut got: Vec<(String, bool)> = Vec::new();
    for (id, base) in &sites {
        if let Some(elig) = facts.fast_stores.get(id) {
            got.push((elig.table.clone(), elig.row_reserve));
            assert_eq!(
                elig.table, *base,
                "the fact's table name must be the store's base name"
            );
            assert!(
                facts.reserve_plans.contains_key(&elig.loop_id),
                "a fast store's loop carries no reserve plan"
            );
        }
    }
    let want: Vec<(String, bool)> = want_stores
        .iter()
        .map(|(t, r)| (t.to_string(), *r))
        .collect();
    assert_eq!(
        got, want,
        "fast-store facts diverged from the pinned lane shape"
    );
    let mut plans: Vec<(Vec<String>, Vec<String>)> = facts
        .reserve_plans
        .values()
        .map(|p| (p.pre_header.clone(), p.nested.clone()))
        .collect();
    plans.sort();
    let want_plans: Vec<(Vec<String>, Vec<String>)> = want_plans
        .iter()
        .map(|(pre, nested)| {
            (
                pre.iter().map(|s| s.to_string()).collect(),
                nested.iter().map(|s| s.to_string()).collect(),
            )
        })
        .collect();
    assert_eq!(plans, want_plans, "reserve plans diverged from the pinned shape");
}

#[test]
fn fast_lane_facts_pin_guard_fill_rides_the_lane() {
    expect_lane(
        "local t = {}\n\
         local i = 0\n\
         while i < 8 do\n\
         \x20 t[i] = i\n\
         \x20 i = i + 1\n\
         end\n",
        &[("t", false)],
        &[(&["t"], &[])],
    );
}

#[test]
fn fast_lane_facts_pin_nested_row_reserves_at_the_store() {
    expect_lane(
        "local t = {{}}\n\
         local i = 0\n\
         while i < 2 do\n\
         \x20 t[0][i] = i\n\
         \x20 i = i + 1\n\
         end\n",
        &[("t", true)],
        &[(&[], &["t"])],
    );
}

#[test]
fn fast_lane_facts_pin_ctor_value_under_a_fill_borrows_the_bound() {
    let source = "local t = {}\n\
                  local i = 0\n\
                  while i < 2 do\n\
                  \x20 t[i] = {}\n\
                  \x20 t[i][0] = i + 3\n\
                  \x20 i = i + 1\n\
                  end\n";
    let (ast, facts) = lane_facts(source).unwrap();
    let mut sites = Vec::new();
    store_sites(&ast, &mut sites);
    assert_eq!(sites.len(), 2, "the golden carries two stores");
    assert!(
        facts.fast_stores.contains_key(&sites[0].0),
        "the guard-keyed constructor store rides the lane"
    );
    assert!(
        !facts.fast_stores.contains_key(&sites[1].0),
        "the literal-keyed row store stays checked"
    );
    let plan = facts
        .reserve_plans
        .values()
        .next()
        .expect("the loop carries a reserve plan");
    assert_eq!(plan.pre_header, vec!["t".to_string()]);
    assert_eq!(plan.nested, vec!["t".to_string()]);
    assert_eq!(
        plan.ctor_reserves,
        std::collections::BTreeSet::from_iter([sites[0].0]),
        "the constructor value's reserve rides its store, keyed by NodeId"
    );
}

#[test]
fn fast_lane_facts_pin_guard_rebind_declines_later_stores() {
    expect_lane(
        "local t = {}\n\
         local i = 0\n\
         while i < 4 do\n\
         \x20 t[i] = i\n\
         \x20 if i == 1 then\n\
         \x20   i = i + 2\n\
         \x20 end\n\
         \x20 t[i] = i\n\
         \x20 i = i + 1\n\
         end\n",
        &[("t", false)],
        &[(&["t"], &[])],
    );
}

#[test]
fn fast_lane_facts_pin_store_after_the_increment_declines() {
    expect_lane(
        "local t = {}\n\
         local i = 0\n\
         while i < 3 do\n\
         \x20 i = i + 1\n\
         \x20 t[i] = i\n\
         end\n",
        &[],
        &[(&["t"], &[])],
    );
}

#[test]
fn fast_lane_facts_pin_guard_shadow_scopes_to_its_block() {
    expect_lane(
        "local t = {}\n\
         local i = 0\n\
         while i < 3 do\n\
         \x20 do\n\
         \x20   local i = 9\n\
         \x20   t[i] = i\n\
         \x20 end\n\
         \x20 t[i] = i\n\
         \x20 i = i + 1\n\
         end\n",
        &[("t", false)],
        &[(&["t"], &[])],
    );
}

#[test]
fn fast_lane_facts_pin_offset_key_declines_in_phase_one() {
    expect_lane(
        "local t = {}\n\
         local j = 1\n\
         while j < 9 do\n\
         \x20 t[j-1] = j - 1\n\
         \x20 j = j + 1\n\
         end\n",
        &[],
        &[],
    );
}
