use crate::analysis;
use crate::ast::{BinOp, Expr, NodeId, Stmt};
use crate::diag::{DiagKind, Diagnostic};
use crate::lexer;
use crate::parser::Parser;
use crate::pipeline::{self, PipelineOpts, Stage};
use logos::Logos;
use std::panic::AssertUnwindSafe;

pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub(crate) fn below(&mut self, n: usize) -> usize {
        debug_assert!(n > 0, "Rng::below called with n == 0");
        ((u128::from(self.next()) * u128::from(n as u64)) >> 64) as usize
    }
}

pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn property_opts(walk_pins: bool) -> PipelineOpts {
    PipelineOpts {
        standalone: false,
        source_file: "property_case.lua".into(),
        walk_pins,
        index_base: 0,
        for_limit_inclusive: false,
    }
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown panic".into())
}

pub(crate) fn diags_text(diags: &[Diagnostic]) -> String {
    diags
        .iter()
        .map(|d| d.render())
        .collect::<Vec<_>>()
        .join("; ")
}

pub(crate) fn compile(source: &str) -> Result<String, (Stage, Vec<Diagnostic>)> {
    compile_with(source, true)
}

pub(crate) fn compile_with(
    source: &str,
    walk_pins: bool,
) -> Result<String, (Stage, Vec<Diagnostic>)> {
    match std::panic::catch_unwind(AssertUnwindSafe(|| {
        pipeline::compile(source, &property_opts(walk_pins))
    })) {
        Ok(r) => r,
        Err(payload) => Err((
            Stage::Panicked,
            vec![Diagnostic::bare(format!(
                "panic: {}",
                panic_message(payload)
            ))],
        )),
    }
}

fn compile_exe(source: &str) -> Result<String, (Stage, Vec<Diagnostic>)> {
    match std::panic::catch_unwind(AssertUnwindSafe(|| {
        pipeline::compile(
            source,
            &PipelineOpts {
                standalone: true,
                source_file: "property_case.lua".into(),
                walk_pins: true,
                index_base: 0,
                for_limit_inclusive: false,
            },
        )
    })) {
        Ok(r) => r,
        Err(payload) => Err((
            Stage::Panicked,
            vec![Diagnostic::bare(format!(
                "panic: {}",
                panic_message(payload)
            ))],
        )),
    }
}

pub(crate) fn compile_tokenized_with(
    source: &str,
    tokens: Vec<(lexer::Token<'_>, usize)>,
    walk_pins: bool,
) -> Result<String, (Stage, Vec<Diagnostic>)> {
    match std::panic::catch_unwind(AssertUnwindSafe(|| {
        pipeline::compile_parts(tokens, source, &property_opts(walk_pins))
    })) {
        Ok(r) => r,
        Err(payload) => Err((
            Stage::Panicked,
            vec![Diagnostic::bare(format!(
                "panic: {}",
                panic_message(payload)
            ))],
        )),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Elem {
    Int,
    Flt,
    Bool,
    Str,
    Any,
}

impl Elem {
    fn esize(self) -> u64 {
        match self {
            Elem::Bool => 1,
            Elem::Any => 16,
            _ => 8,
        }
    }

    fn kind_flag(self) -> u8 {
        match self {
            Elem::Int => 0,
            Elem::Flt => 2,
            Elem::Bool => 4,
            Elem::Str => 6,
            Elem::Any => 8,
        }
    }

    fn storage(self) -> Option<&'static str> {
        match self {
            Elem::Int => Some("i64"),
            Elem::Flt => Some("double"),
            Elem::Bool => Some("i8"),
            Elem::Str => Some("ptr"),
            Elem::Any => None,
        }
    }

    fn is_any(self) -> bool {
        self == Elem::Any
    }

    fn join(self, other: Elem) -> Elem {
        if self == other { self } else { Elem::Any }
    }

    fn lit(self, rng: &mut Rng) -> &'static str {
        match self {
            Elem::Int => ["3", "7"][rng.below(2)],
            Elem::Flt => ["2.5", "0.5"][rng.below(2)],
            Elem::Bool => ["true", "false"][rng.below(2)],
            Elem::Str => ["\"s\"", "\"w\""][rng.below(2)],
            Elem::Any => unreachable!("literals are per-kind, Any only arises by joining"),
        }
    }
}

fn witness_kind(rng: &mut Rng) -> Elem {
    [Elem::Int, Elem::Flt, Elem::Bool, Elem::Str][rng.below(4)]
}

fn pick_witness_kind(rng: &mut Rng, locked: Option<Elem>) -> Elem {
    locked.unwrap_or_else(|| witness_kind(rng))
}

fn record_witness(kind: Elem, elem: &mut Option<Elem>) {
    *elem = Some(match *elem {
        Some(e) => e.join(kind),
        None => kind,
    });
}

fn elem_name(elem: Elem) -> &'static str {
    match elem {
        Elem::Int => "Int",
        Elem::Flt => "Flt",
        Elem::Bool => "Bool",
        Elem::Str => "Str",
        Elem::Any => "Any",
    }
}

pub(crate) fn gen_program(rng: &mut Rng) -> (String, Elem) {
    let mut elem: Option<Elem> = None;
    let mut lines: Vec<String> = Vec::new();

    let locked: Option<Elem> = if rng.below(20) < 9 {
        Some(witness_kind(rng))
    } else {
        None
    };

    if rng.below(5) == 0 {
        lines.push("local t = {}".to_string());
        let k0 = pick_witness_kind(rng, locked);
        record_witness(k0, &mut elem);
        lines.push(format!("t[{}] = {}", rng.below(8), k0.lit(rng)));
    } else {
        let mixed = locked.is_none() && rng.below(4) == 0;
        let n = 1 + rng.below(2);
        let k0 = pick_witness_kind(rng, locked);
        let entries: Vec<String> = (0..n)
            .map(|i| {
                let k = if mixed && i == 1 {
                    pick_witness_kind(rng, locked)
                } else {
                    k0
                };
                record_witness(k, &mut elem);
                k.lit(rng).to_string()
            })
            .collect();
        lines.push(format!("local t = {{{}}}", entries.join(", ")));
    }

    let stmts = 2 + rng.below(6);
    for _ in 0..stmts {
        match rng.below(11) {
            0 => {
                let k = pick_witness_kind(rng, locked);
                record_witness(k, &mut elem);
                lines.push(format!("t[{}] = {}", rng.below(8), k.lit(rng)));
            }
            1 | 2 => lines.push(format!("print(t[{}])", rng.below(8))),
            3 => lines.push("print(#t)".to_string()),
            4 | 5 => {
                lines.push(ctor_line(rng, locked, &mut elem));
            }
            6 => {
                lines.push("t = nil".to_string());
                lines.push(ctor_line(rng, locked, &mut elem));
            }
            7 => {
                let body = branch_body(rng, locked, &mut elem);
                if rng.below(2) == 0 {
                    let cond = ["true", "false"][rng.below(2)];
                    lines.push(format!("if {cond} then\n{body}\nend"));
                } else {
                    lines.push(format!("do\n{body}\nend"));
                }
            }
            8 | 9 => {
                let cond = ["true", "false"][rng.below(2)];
                let then_body = branch_body(rng, locked, &mut elem);
                let else_body = branch_body(rng, locked, &mut elem);
                lines.push(format!(
                    "if {cond} then\n{then_body}\nelse\n{else_body}\nend"
                ));
            }
            _ => {
                let bound = 2 + rng.below(4);
                match rng.below(3) {
                    0 => {
                        let k = pick_witness_kind(rng, locked);
                        record_witness(k, &mut elem);
                        lines.push(format!(
                            "for i = 0, {bound} do\n  t[i] = {}\nend",
                            k.lit(rng)
                        ));
                    }
                    1 => lines.push(format!("for i = 0, {bound} do\n  print(t[i])\nend")),
                    _ => {
                        let k = pick_witness_kind(rng, locked);
                        record_witness(k, &mut elem);
                        let cond = ["true", "false"][rng.below(2)];
                        lines.push(format!(
                            "while {cond} do\n  t[{}] = {}\n  break\nend",
                            rng.below(8),
                            k.lit(rng)
                        ));
                    }
                }
            }
        }
    }

    lines.push(format!("print(t[{}])", rng.below(8)));

    (
        lines.join("\n") + "\n",
        elem.expect("the opening sequence always witnesses at least one kind"),
    )
}

fn branch_body(rng: &mut Rng, locked: Option<Elem>, elem: &mut Option<Elem>) -> String {
    match rng.below(4) {
        0 => {
            let k = pick_witness_kind(rng, locked);
            record_witness(k, elem);
            format!("t[{}] = {}", rng.below(8), k.lit(rng))
        }
        1 => format!("print(t[{}])", rng.below(8)),
        2 => ctor_line(rng, locked, elem),
        _ => {
            let rebind = ctor_line(rng, locked, elem);
            format!("t = nil\n{rebind}")
        }
    }
}

fn ctor_line(rng: &mut Rng, locked: Option<Elem>, elem: &mut Option<Elem>) -> String {
    let n = rng.below(3);
    if n == 0 {
        return "t = {}".to_string();
    }
    let k0 = pick_witness_kind(rng, locked);
    let entries: Vec<String> = (0..n)
        .map(|i| {
            let k = if i == 1 && locked.is_none() && rng.below(2) == 0 {
                pick_witness_kind(rng, locked)
            } else {
                k0
            };
            record_witness(k, elem);
            k.lit(rng).to_string()
        })
        .collect();
    format!("t = {{{}}}", entries.join(", "))
}

fn assert_elem_congruence(ll: &str, expected: Elem, context: &str) {
    let mut news = 0usize;
    let mut accesses = 0usize;
    let mut seams = 0usize;
    for line in ll.lines() {
        if line.contains("ptrtoint") {
            continue;
        }
        if let Some(rest) = line.split("@alm_tbl_new(i64 ").nth(1) {
            news += 1;
            let esize: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            let esize: u64 = esize.parse().unwrap_or(u64::MAX);
            assert_eq!(
                esize,
                expected.esize(),
                "{context}: alm_tbl_new allocated esize {esize}, expected {} for elem {}\
                 \noffending line: {line}",
                expected.esize(),
                elem_name(expected),
            );
            let flags: String = rest
                .split(", i8 ")
                .nth(1)
                .unwrap_or("missing")
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            let flags: u8 = flags.trim().parse().unwrap_or(u8::MAX);
            assert_eq!(
                (flags >> 1) & 0x07,
                expected.kind_flag() >> 1,
                "{context}: alm_tbl_new flags byte names elem kind {}, expected {} for elem {}\
                 \noffending line: {line}",
                (flags >> 1) & 0x07,
                expected.kind_flag() >> 1,
                elem_name(expected),
            );
            assert_eq!(
                flags & 0x80,
                0,
                "{context}: flat scalar program allocated a nested-table ctor\
                 \noffending line: {line}"
            );
        }
        if line.contains("call") && line.contains("@alm_tbl_") {
            let sym_start = line.find("@alm_tbl_").expect("checked above");
            let tail = &line[sym_start..];
            let sym: &str = &tail[..tail.find('(').unwrap_or(tail.len())];
            if sym.starts_with("@alm_tbl_get") || sym.starts_with("@alm_tbl_set") {
                accesses += 1;
                assert_eq!(
                    sym.contains("_any"),
                    expected.is_any(),
                    "{context}: access face {sym} is {} family, expected {} for elem {}\
                     \noffending line: {line}",
                    if sym.contains("_any") {
                        "an Any"
                    } else {
                        "a scalar"
                    },
                    if expected.is_any() {
                        "the Any"
                    } else {
                        "the scalar"
                    },
                    elem_name(expected),
                );
                if sym == "@alm_tbl_get" {
                    let tail_size: String = line
                        .rsplit("i64 ")
                        .next()
                        .unwrap_or("missing")
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    let tail_size: u64 = tail_size.trim().parse().unwrap_or(u64::MAX);
                    assert_eq!(
                        tail_size,
                        expected.esize(),
                        "{context}: scalar alm_tbl_get names cell size {tail_size}, expected {} \
                         for elem {}\noffending line: {line}",
                        expected.esize(),
                        elem_name(expected),
                    );
                }
            }
        }
        if line.contains(" = alloca ") && (line.contains(".dst") || line.contains(".valp")) {
            seams += 1;
            let storage = line.rsplit("alloca ").next().unwrap_or("").trim();
            match expected.storage() {
                Some(want) => assert_eq!(
                    storage,
                    want,
                    "{context}: cell seam allocates `{storage}`, expected `{want}` for elem {}\
                     \noffending line: {line}",
                    elem_name(expected),
                ),
                None => panic!(
                    "{context}: scalar cell seam under an Any expectation — Any cells ride \
                     i128 register faces and must never touch a cell seam\
                     \noffending line: {line}"
                ),
            }
        }
    }
    assert!(
        news >= 1,
        "{context}: the artifact contains no alm_tbl_new — the ctor never lowered"
    );
    assert!(
        accesses >= 1,
        "{context}: the artifact contains no @alm_tbl_get*/@alm_tbl_set* calls — \
         table accesses never lowered"
    );
    if !expected.is_any() {
        assert!(
            seams >= 1,
            "{context}: the artifact contains no scalar cell seams for elem {} — \
             the accesses never touched a cell",
            elem_name(expected)
        );
    }
}

fn report(source: &str, expected: Elem, detail: String) -> String {
    format!(
        "{detail}\nexpected elem: {}\nprogram:\n{}",
        elem_name(expected),
        source
            .lines()
            .enumerate()
            .map(|(i, l)| format!("  {:2}| {}", i + 1, l))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

pub(crate) fn indent(source: &str) -> String {
    source
        .lines()
        .map(|l| format!("  {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_parts(source: &str) -> Vec<Stmt> {
    let mut tokens = Vec::new();
    let mut offsets = Vec::new();
    let mut lex = lexer::Token::lexer(source);
    loop {
        match lex.next() {
            Some(Ok(token)) => {
                offsets.push(lex.span().start);
                tokens.push(token);
            }
            Some(Err(_)) => panic!("H5 pin source failed to lex"),
            None => break,
        }
    }
    let mut parser = Parser::new(tokens.into_iter().zip(offsets).collect(), source);
    let ast = parser.parse_program();
    assert!(
        parser.diagnostics.is_empty(),
        "H5 pin source failed to parse: {:?}",
        parser.diagnostics
    );
    ast
}

fn parse_parts_with(source: &str, for_limit_inclusive: bool) -> Vec<Stmt> {
    let mut tokens = Vec::new();
    let mut offsets = Vec::new();
    let mut lex = lexer::Token::lexer(source);
    loop {
        match lex.next() {
            Some(Ok(token)) => {
                offsets.push(lex.span().start);
                tokens.push(token);
            }
            Some(Err(_)) => panic!("H5 pin source failed to lex"),
            None => break,
        }
    }
    let mut parser = Parser::new(tokens.into_iter().zip(offsets).collect(), source)
        .with_for_limit_inclusive(for_limit_inclusive);
    let ast = parser.parse_program();
    assert!(
        parser.diagnostics.is_empty(),
        "H5 pin source failed to parse: {:?}",
        parser.diagnostics
    );
    ast
}

include!("generated_programs_keep_one_elem_truth.rs");
include!("generator_sweep_survives_fresh_seeds.rs");
include!("hand_pinned_shapes_match_the_any_law.rs");
include!("rebind_adoption_probes_stay_pinned.rs");
include!("walk_pin_len_deferral_rejects_identically_pin.rs");
include!("walk_pin_computed_scalars_stay_legal_pin.rs");
include!("walk_pin_unary_neg_scalar_stays_legal_pin.rs");
include!("w3a_crossing_faces_reject_on_both_drive_and_spawn.rs");
include!("h5_for_desugar_reports_at_the_header_pin.rs");
include!("h5_for_desugar_inclusive_latch_pin.rs");
include!("h5_one_to_one_sugars_and_elseif_attribution_pin.rs");
include!("h5_census_trips_on_right_count_wrong_line.rs");
include!("h8_elaborator_unit_goldens_pin.rs");
include!("h8_tuple_assign_attribution_pin.rs");
include!("h8_census_trips_on_tuple_wrong_line.rs");
include!("h7_one_pass_pairing_and_faces_pin.rs");
include!("h7_fn_touched_rides_the_pass_pin.rs");
include!("h7_census_sees_through_field_pin.rs");
include!("h11_one_formatter_shipped_path_pin.rs");
include!("h11_site_derives_the_prefix_pin.rs");
include!("h11_kinds_and_desync_render_law_pin.rs");
include!("h6_identity_rides_the_node_pin.rs");
include!("h6_id_census_pin.rs");
include!("h6_maps_key_ids_not_addresses_pin.rs");
include!("h12_instance_mint_law_pin.rs");
include!("h12_shadow_golden_depth_two_pin.rs");
include!("h12_bridge_totality_pin.rs");
include!("h12_shadow_maps_empty_by_site_law_pin.rs");
include!("h12_break_base_is_per_event_pin.rs");
include!("h12_multi_event_lowers_clean_at_equal_depth_pin.rs");
include!("h12_local_types_key_the_outermost_extent_pin.rs");
include!("h12_instantiation_facts_cover_every_site_pin.rs");
include!("h12_dispatch_mints_at_absolute_base_pin.rs");
include!("monomorphic_blocks_keep_one_elem_truth.rs");
include!("h12_multi_event_divergent_depth_lowers_clean_pin.rs");
include!("h12_break_arm_consumes_the_event_base_pin.rs");
include!("h12_nested_dispatch_rides_the_outer_owner_pin.rs");
include!("h12_creation_envs_site_law_at_depth_two_pin.rs");
include!("h12_three_events_mint_in_body_order_pin.rs");
include!("h12_lowerer_consumes_the_records_pin.rs");
include!("h12_recursion_guard_reads_the_instance_chain_pin.rs");
include!("h12_loop_truth_is_sugar_blind_pin.rs");
include!("fast_lane_facts_pin.rs");
