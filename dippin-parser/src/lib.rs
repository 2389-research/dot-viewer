// ABOUTME: Public API for the dippin parser and DOT exporter.
// ABOUTME: Provides parse() and export_dot() functions for converting .dip files to DOT format.

pub mod duration;
pub mod error;
pub mod export_dot;
pub mod ir;
pub mod lexer;
pub mod parser;
pub mod validate;

pub use duration::Duration;
pub use error::{Diagnostic, DiagnosticKind, Error, Result, Severity};
pub use export_dot::{export_dot as export_dot_string, ExportOptions};
pub use ir::Workflow;
pub use parser::Parser;

/// Parse a dippin source string into a Workflow IR.
pub fn parse(source: &str, filename: &str) -> Result<Workflow> {
    Parser::new(source, filename).parse()
}

/// Convert a dippin source string directly to DOT format.
pub fn convert_to_dot(source: &str, filename: &str) -> Result<String> {
    let wf = parse(source, filename)?;
    Ok(export_dot_string(&wf, &ExportOptions::default()))
}

/// Convert a dippin source string to DOT format with options.
pub fn convert_to_dot_with_options(
    source: &str,
    filename: &str,
    opts: &ExportOptions,
) -> Result<String> {
    let wf = parse(source, filename)?;
    Ok(export_dot_string(&wf, opts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_dot_minimal() {
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
        let dot = convert_to_dot(input, "test.dip").expect("should convert");
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

        let dot = export_dot_string(&wf, &ExportOptions::default());
        assert!(dot.contains("digraph MultiProvider {"));
        assert!(dot.contains("Think"));
        assert!(dot.contains("Generate"));
        assert!(dot.contains("Think -> Generate"));
    }
}
