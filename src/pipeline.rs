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
}

#[derive(Debug)]
pub enum Stage {
    #[cfg(test)]
    Panicked,
    Front,
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
            Stage::Ffi => "ffi gate (exe-only surface)",
            Stage::Analyze => "shape analyzer + checker (one walk)",
            Stage::Lower => "lowerer",
            Stage::Backend => "backend",
        }
    }
}

const FFI_EXE_ONLY: &str = "alm compiler error: the ffi surface (require(\"ffi\") / ffi.load / \
                            lib.sym) is --exe-only — rebuild with --exe; a shared object loading \
                            shared objects is not acyclic destiny";

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
    if uses_ffi(&ast) && !opts.standalone {
        return Err((Stage::Ffi, vec![Diagnostic::bare(FFI_EXE_ONLY)]));
    }

    let ctx = analysis::build_context(&ast);

    let shape = shape::analyze_with_base(&ctx, opts.walk_pins, opts.index_base);
    if !shape.diagnostics.is_empty() {
        return Err((Stage::Analyze, shape.diagnostics));
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

fn uses_ffi(stmts: &[Stmt]) -> bool {
    use crate::ast::NodeId;
    use crate::visit::{Visitor, walk_stmts};
    use std::convert::Infallible;

    struct FfiUsed(bool);
    impl Visitor for FfiUsed {
        type Break = Infallible;
        fn visit_ffi_module(&mut self, _id: NodeId) -> Result<(), Self::Break> {
            self.0 = true;
            Ok(())
        }
    }
    let mut v = FfiUsed(false);
    let _ = walk_stmts(&mut v, stmts);
    v.0
}
