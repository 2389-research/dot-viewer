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

/// Maximum source file size in bytes accepted by `parse`.
pub const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024;

/// Parse a dippin source string into a Workflow IR.
pub fn parse(source: &str, filename: impl AsRef<std::path::Path>) -> Result<Workflow> {
    let filename = filename.as_ref().to_string_lossy().into_owned();
    if source.len() > MAX_INPUT_SIZE {
        return Err(Error::Parse {
            file: filename.clone(),
            diagnostics: vec![Diagnostic::error(
                DiagnosticKind::Other,
                format!("input exceeds maximum size of {} bytes", MAX_INPUT_SIZE),
                crate::ir::SourceLocation {
                    file: filename,
                    line: 1,
                    column: 1,
                },
            )],
        });
    }
    Parser::new(source, &filename).parse()
}

/// Parse and convert to DOT in a single call.
pub fn parse_to_dot(source: &str, filename: impl AsRef<std::path::Path>) -> Result<String> {
    parse_to_dot_with_options(source, filename, &ExportOptions::default())
}

/// Parse and convert to DOT with custom export options.
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
