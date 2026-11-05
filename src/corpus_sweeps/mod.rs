use std::fs;
use std::path::Path;

use crate::diag::Diagnostic;
use crate::lexer;
use crate::pipeline::Stage;
use crate::property_tests::{
    Rng, compile, compile_tokenized_with, compile_with, diags_text, fnv1a, gen_program, indent,
};
use logos::Logos;

include!(concat!(env!("OUT_DIR"), "/corpus_manifest.rs"));

fn corpus_sources() -> Vec<(String, String)> {
    let cases_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("cases");
    let mut on_disk: Vec<String> = fs::read_dir(&cases_dir)
        .unwrap_or_else(|e| panic!("corpus_sweeps: failed to read {}: {e}", cases_dir.display()))
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".lua"))
        .collect();
    on_disk.sort();
    let manifest: Vec<&str> = CORPUS_CASES.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        on_disk, manifest,
        "cases/ drifted from the generated corpus manifest — a cargo build regenerates \
         it, so a surviving drift means a case was edited under a running test binary \
         or a stray file landed in cases/: investigate, never re-blind"
    );
    CORPUS_CASES
        .iter()
        .map(|(name, _)| {
            let source = fs::read_to_string(cases_dir.join(name))
                .unwrap_or_else(|e| panic!("corpus_sweeps: cases/{name}: {e}"));
            (name.to_string(), source)
        })
        .collect()
}

include!("corpus_mutations_never_desync_the_walks.rs");
include!("token_soup_never_panics_or_desyncs.rs");
include!("str_key_derives_agree_across_the_abi.rs");
include!("walk_pins_are_a_pure_subset_over_the_corpus.rs");
include!("fast_lane_facts_hold_over_the_corpus.rs");
