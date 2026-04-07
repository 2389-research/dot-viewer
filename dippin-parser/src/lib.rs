//! # dippin-parser
//!
//! A Rust parser and DOT exporter for the Dippin DSL — a higher-level authoring
//! format for AI agent workflows that lowers to Graphviz DOT for visualization.
//!
//! This crate is a port of the upstream Go implementation at
//! [github.com/2389-research/dippin-lang](https://github.com/2389-research/dippin-lang).
//! See that repository for the canonical language reference.
//!
//! ## Quick start
//!
//! ```
//! use dippin_parser::{parse, parse_to_dot};
//!
//! let source = "\
//! workflow Greet
//!   start: Ask
//!   exit: Done
//!   agent Ask
//!     prompt: \"Hi!\"
//!     model: claude-sonnet-4-6
//!     provider: anthropic
//!   agent Done
//!     prompt: \"Bye!\"
//!     model: gpt-4.1-nano
//!     provider: openai
//!   edges
//!     Ask -> Done
//! ";
//!
//! let wf = parse(source, "greet.dip").unwrap();
//! assert_eq!(wf.name, "Greet");
//!
//! let dot = parse_to_dot(source, "greet.dip").unwrap();
//! assert!(dot.contains("digraph Greet {"));
//! ```
//!
//! ## Features
//!
//! - `serde` — derives `Serialize`/`Deserialize` for IR types.
//!
//! ## Stability
//!
//! Pre-1.0. All public types are `#[non_exhaustive]`.

// ABOUTME: Public API for the dippin parser and DOT exporter.
// ABOUTME: Provides parse() and export_dot() functions for converting .dip files to DOT format.

pub mod duration;
pub mod error;
pub mod export_dot;
pub mod ir;
pub(crate) mod lexer;
pub(crate) mod parser;
pub mod validate;

pub use duration::Duration;
pub use error::{Diagnostic, DiagnosticKind, Error, Result, Severity};
pub use export_dot::{ExportOptions, RankDir};
pub use ir::{
    AgentConfig, BranchConfig, Condition, Edge, FanInConfig, HumanConfig, Node, NodeConfig,
    NodeIO, NodeKind, ParallelConfig, RetryConfig, SourceLocation, StyleSelector, StylesheetRule,
    SubgraphConfig, ToolConfig, Workflow, WorkflowDefaults,
};
use parser::Parser;

/// Maximum source file size in bytes accepted by [`parse`].
pub const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024;

/// Parse a Dippin source string into a [`Workflow`].
///
/// # Arguments
///
/// * `source` — the `.dip` source text.
/// * `filename` — used in diagnostics; pass any path-like value.
///
/// # Errors
///
/// Returns [`Error::Parse`] containing one or more [`Diagnostic`]s if the source
/// has syntax errors, references undefined nodes, or exceeds [`MAX_INPUT_SIZE`].
///
/// # Examples
///
/// ```
/// use dippin_parser::parse;
/// let src = "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
/// let wf = parse(src, "t.dip").unwrap();
/// assert_eq!(wf.name, "F");
/// ```
pub fn parse(source: &str, filename: impl AsRef<std::path::Path>) -> Result<Workflow> {
    let filename = filename.as_ref().to_string_lossy().into_owned();
    if source.len() > MAX_INPUT_SIZE {
        let file: std::sync::Arc<str> = std::sync::Arc::from(filename);
        return Err(Error::Parse {
            file: std::sync::Arc::clone(&file),
            diagnostics: vec![Diagnostic::error(
                DiagnosticKind::Other,
                format!("input exceeds maximum size of {} bytes", MAX_INPUT_SIZE),
                crate::ir::SourceLocation {
                    file,
                    line: 1,
                    column: 1,
                },
            )],
        });
    }
    Parser::new(source, &filename).parse()
}

/// Parse a Dippin source string and emit Graphviz DOT in a single call.
///
/// Equivalent to calling [`parse`] followed by [`Workflow::to_dot`] with
/// [`ExportOptions::default`].
///
/// # Arguments
///
/// * `source` — the `.dip` source text.
/// * `filename` — used in diagnostics; pass any path-like value.
///
/// # Errors
///
/// Returns [`Error::Parse`] if the source fails to parse. See [`parse`] for
/// the full set of failure modes.
///
/// # Examples
///
/// ```
/// use dippin_parser::parse_to_dot;
/// let src = "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
/// let dot = parse_to_dot(src, "t.dip").unwrap();
/// assert!(dot.contains("digraph F {"));
/// ```
pub fn parse_to_dot(source: &str, filename: impl AsRef<std::path::Path>) -> Result<String> {
    parse_to_dot_with_options(source, filename, &ExportOptions::default())
}

/// Parse a Dippin source string and emit Graphviz DOT using the given
/// [`ExportOptions`].
///
/// # Arguments
///
/// * `source` — the `.dip` source text.
/// * `filename` — used in diagnostics; pass any path-like value.
/// * `opts` — export tuning (rank direction, prompt inclusion, etc.).
///
/// # Errors
///
/// Returns [`Error::Parse`] if the source fails to parse. See [`parse`] for
/// the full set of failure modes.
///
/// # Examples
///
/// ```
/// use dippin_parser::{parse_to_dot_with_options, ExportOptions, RankDir};
/// let src = "workflow F\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
/// let mut opts = ExportOptions::default();
/// opts.rank_dir = RankDir::LeftRight;
/// let dot = parse_to_dot_with_options(src, "t.dip", &opts).unwrap();
/// assert!(dot.contains("rankdir=LR"));
/// ```
pub fn parse_to_dot_with_options(
    source: &str,
    filename: impl AsRef<std::path::Path>,
    opts: &ExportOptions,
) -> Result<String> {
    let wf = parse(source, filename)?;
    Ok(wf.to_dot(opts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oversize_input_rejected() {
        let big = "a".repeat(crate::MAX_INPUT_SIZE + 1);
        assert!(crate::parse(&big, "big.dip").is_err());
    }

    #[test]
    fn test_parse_to_dot_minimal() {
        let input = r#"workflow Minimal
  goal: "Test workflow"
  start: Ask
  exit: Done

  human Ask
    mode: freeform

  agent Done
    prompt:
      Complete the task.

  edges
    Ask -> Done
"#;
        let dot = parse_to_dot(input, "test.dip").expect("should convert");
        assert!(dot.contains("digraph Minimal {"));
        assert!(dot.contains("Ask"));
        assert!(dot.contains("Done"));
        assert!(dot.contains("Ask -> Done"));
    }

    #[test]
    fn test_parse_and_export_round_trip() {
        let input = r#"workflow MultiProvider
  goal: "Test workflow with multiple providers"
  start: Ask
  exit: Done

  human Ask
    mode: freeform

  agent Think
    label: "Think Step"
    model: claude-sonnet-4-6
    provider: anthropic
    prompt:
      Analyze the request.

  agent Generate
    label: "Generate Step"
    model: gpt-4.1-nano
    provider: openai
    prompt:
      Generate a response.

  agent Done
    prompt:
      Finish up.

  edges
    Ask -> Think
    Think -> Generate
    Generate -> Done
"#;
        let wf = parse(input, "test.dip").expect("should parse");
        assert_eq!(wf.name, "MultiProvider");
        assert_eq!(wf.nodes.len(), 4);
        assert_eq!(wf.edges.len(), 3);

        let dot = wf.to_dot(&ExportOptions::default());
        assert!(dot.contains("digraph MultiProvider {"));
        assert!(dot.contains("Think"));
        assert!(dot.contains("Generate"));
        assert!(dot.contains("Think -> Generate"));
    }
}
