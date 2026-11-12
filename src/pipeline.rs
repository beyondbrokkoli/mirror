use crate::analysis;
use crate::ast::Stmt;
use crate::backend;
use crate::diag::Diagnostic;
use crate::ir;
use crate::lexer;
use crate::lowerer;
use crate::parser::Parser;
use crate::shape;
use logos::Logos;

pub struct PipelineOpts {
    pub standalone: bool,
    pub source_file: std::path::PathBuf,
    pub walk_pins: bool,
    /// Surface index base (0 default, 1 for Lua-style). Physical key =
    /// user key − index_base at every seam.
    pub index_base: i64,
    /// Surface for-limit mode. False (default) is exclusive: the guard
    /// is `i < latched-limit`. True is inclusive: the latch is
    /// `limit + 1` and the guard never changes — desugar-tier only,
    /// it never reaches the analyzer or the lowerer.
    pub for_limit_inclusive: bool,
    /// Where dotted requires anchor (`--module-root`). None leaves a
    /// program with requires a named rejection away.
    pub module_root: Option<std::path::PathBuf>,
}

#[derive(Debug)]
pub enum Stage {
    #[cfg(test)]
    Panicked,
    Front,
    Modules,
    Ffi,
    Analyze,
    Lower,
    Backend,
}

impl Stage {
    #[cfg(test)]
    pub fn name(&self) -> &'static str {
        match self {
            Stage::Panicked => "panicked",
            Stage::Front => "front (lexer/parser)",
            Stage::Modules => "require splice (module amalgamation)",
            Stage::Ffi => "ffi gate (ffi.load is --exe-only)",
            Stage::Analyze => "shape analyzer + checker (one walk)",
            Stage::Lower => "lowerer",
            Stage::Backend => "backend",
        }
    }
}

const FFI_LOAD_EXE_ONLY: &str = "alm compiler error: ffi.load is --exe-only — a shared object \
                                 may not load shared objects (the callee seam: a .so exposes \
                                 alm_arg_kind/alm_exec, it never loads)";

pub fn compile(source: &str, opts: &PipelineOpts) -> Result<String, (Stage, Vec<Diagnostic>)> {
    let mut tokens = Vec::new();
    let mut offsets = Vec::new();
    let mut front_diagnostics: Vec<Diagnostic> = Vec::new();
    let mut lexer = lexer::Token::lexer(source);
    loop {
        match lexer.next() {
            Some(Ok(token)) => {
                offsets.push(lexer.span().start);
                tokens.push(token);
            }
            Some(Err(_)) => {
                let line = 1 + source.as_bytes()[..lexer.span().start]
                    .iter()
                    .filter(|b| **b == b'\n')
                    .count();
                front_diagnostics.push(Diagnostic::syntax("Lexer error").with_site(Some(line)));
                break;
            }
            None => break,
        }
    }

    let mut parser = Parser::new(tokens.into_iter().zip(offsets).collect(), source)
        .with_index_base(opts.index_base)
        .with_for_limit_inclusive(opts.for_limit_inclusive);
    let ast = parser.parse_program();
    front_diagnostics.extend(parser.diagnostics);

    if !front_diagnostics.is_empty() {
        return Err((Stage::Front, front_diagnostics));
    }

    post_front(ast, opts)
}

#[cfg(test)]
pub fn compile_parts(
    tokens: Vec<(lexer::Token<'_>, usize)>,
    source: &str,
    opts: &PipelineOpts,
) -> Result<String, (Stage, Vec<Diagnostic>)> {
    let mut parser = Parser::new(tokens, source);
    let ast = parser.parse_program();
    if !parser.diagnostics.is_empty() {
        return Err((Stage::Front, parser.diagnostics));
    }
    post_front(ast, opts)
}

fn post_front(ast: Vec<Stmt>, opts: &PipelineOpts) -> Result<String, (Stage, Vec<Diagnostic>)> {
    // The splice runs ahead of the ffi gate on purpose: inlining is
    // compile-time and pure, and the amalgamated unit carries ffi only
    // if the source does — then the law applies to the whole unit
    // as-is (M3 decision 8; M4 relaxed the law from "any ffi" to
    // "ffi.load", resolved by the checker's verb table below).
    let ast = crate::require::expand(
        ast,
        crate::require::RequireOpts {
            module_root: opts.module_root.clone(),
            index_base: opts.index_base,
            for_limit_inclusive: opts.for_limit_inclusive,
        },
    )
    .map_err(|d| (Stage::Modules, d))?;

    let ctx = analysis::build_context(&ast);

    let shape = shape::analyze_with_base(&ctx, opts.walk_pins, opts.index_base);
    if !shape.diagnostics.is_empty() {
        return Err((Stage::Analyze, shape.diagnostics));
    }

    // The acyclicity law, as link-time accounting: a .so may use the
    // callee seam (cdef/new/sizeof — the M4 relaxation), but it may
    // never itself load a shared object. Loading stays --exe-only;
    // the verbs are the checker's resolved table, so the gate names
    // the actual load, not any ffi mention.
    if !opts.standalone
        && shape
            .ffi_verbs
            .values()
            .any(|verb| matches!(verb, crate::shape::FfiVerb::Load))
    {
        return Err((Stage::Ffi, vec![Diagnostic::bare(FFI_LOAD_EXE_ONLY)]));
    }

    let mut ir_lowerer = lowerer::IrLowerer::new(&shape);
    let mut ir_program = ir_lowerer.lower_program(&ast);
    if !ir_lowerer.diagnostics.is_empty() {
        return Err((Stage::Lower, ir_lowerer.diagnostics));
    }

    ir_program.source_file = opts.source_file.clone();
    if opts.standalone {
        ir_program.entry = ir::EntryKind::Exe {
            args: references_arg(&ast),
        };
    }

    backend::generate_llvm_ir(&ir_program).map_err(|d| (Stage::Backend, d))
}

fn references_arg(stmts: &[Stmt]) -> bool {
    use crate::ast::NodeId;
    use crate::visit::{Visitor, walk_stmts};
    use std::convert::Infallible;

    struct ArgRef(bool);
    impl Visitor for ArgRef {
        type Break = Infallible;
        fn visit_identifier(&mut self, name: &str, _id: NodeId) -> Result<(), Self::Break> {
            self.0 |= name == "arg";
            Ok(())
        }
    }
    let mut v = ArgRef(false);
    let _ = walk_stmts(&mut v, stmts);
    v.0
}
