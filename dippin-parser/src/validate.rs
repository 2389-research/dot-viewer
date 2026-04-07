// ABOUTME: Semantic validation pass run after parsing.
// ABOUTME: Verifies start/exit/edge references point at declared nodes.

use std::collections::HashSet;

use crate::error::{Diagnostic, DiagnosticKind};
use crate::ir::{SourceLocation, Workflow};

/// Run semantic validation on a parsed workflow and return any diagnostics.
pub fn validate(wf: &Workflow, file: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let ids: HashSet<&str> = wf.nodes.iter().map(|n| n.id.as_str()).collect();

    let mut check = |role: &str, target: &str, line: usize| {
        if !target.is_empty() && !ids.contains(target) {
            diags.push(Diagnostic::error(
                DiagnosticKind::UndefinedNodeReference(target.to_string()),
                format!("{} references undefined node `{}`", role, target),
                SourceLocation {
                    file: file.to_string(),
                    line,
                    column: 1,
                },
            ));
        }
    };

    check("workflow.start", &wf.start, 1);
    check("workflow.exit", &wf.exit, 1);

    for edge in &wf.edges {
        check("edge `from`", &edge.from, edge.source.line);
        check("edge `to`", &edge.to, edge.source.line);
    }

    diags
}
