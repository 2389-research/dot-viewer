// ABOUTME: Semantic validation pass run after parsing.
// ABOUTME: Verifies start/exit/edge references point at declared nodes.

use std::collections::HashSet;

use crate::error::{Diagnostic, DiagnosticKind};
use crate::ir::{NodeConfig, SourceLocation, Workflow};

/// Run semantic validation on a parsed workflow and return any diagnostics.
pub fn validate(wf: &Workflow, file: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let ids: HashSet<&str> = wf.nodes.iter().map(|n| n.id.as_str()).collect();

    let push = |diags: &mut Vec<Diagnostic>, role: &str, target: &str, line: usize| {
        diags.push(Diagnostic::error(
            DiagnosticKind::UndefinedNodeReference(target.to_string()),
            format!("{} references undefined node `{}`", role, target),
            SourceLocation {
                file: file.to_string(),
                line,
                column: 1,
            },
        ));
    };

    let check = |diags: &mut Vec<Diagnostic>, role: &str, target: &str, line: usize| {
        if !target.is_empty() && !ids.contains(target) {
            push(diags, role, target, line);
        }
    };

    check(&mut diags, "workflow.start", &wf.start, 1);
    check(&mut diags, "workflow.exit", &wf.exit, 1);
    check(
        &mut diags,
        "workflow.defaults.restart_target",
        &wf.defaults.restart_target,
        1,
    );

    for edge in &wf.edges {
        check(&mut diags, "edge `from`", &edge.from, edge.source.line);
        check(&mut diags, "edge `to`", &edge.to, edge.source.line);
    }

    for node in &wf.nodes {
        let line = node.source.line;
        check(
            &mut diags,
            "retry.retry_target",
            &node.retry.retry_target,
            line,
        );
        check(
            &mut diags,
            "retry.fallback_target",
            &node.retry.fallback_target,
            line,
        );

        match &node.config {
            NodeConfig::Parallel(cfg) => {
                for target in &cfg.targets {
                    check(&mut diags, "parallel target", target, line);
                }
                for branch in &cfg.branches {
                    check(&mut diags, "parallel branch target", &branch.target, line);
                }
            }
            NodeConfig::FanIn(cfg) => {
                for source in &cfg.sources {
                    check(&mut diags, "fan_in source", source, line);
                }
            }
            _ => {}
        }
    }

    diags
}

#[cfg(test)]
mod tests {
    use crate::DiagnosticKind;

    #[test]
    fn parallel_targets_must_exist() {
        let src = "workflow F\n  start: S\n  exit: E\n  agent S\n    prompt: x\n    model: m\n    provider: p\n  parallel P -> Missing\n  agent E\n    prompt: y\n    model: m\n    provider: p\n  edges\n    S -> P\n    P -> E\n";
        let err = crate::parse(src, "t.dip").unwrap_err();
        assert!(err.diagnostics().iter().any(|d| matches!(
            &d.kind,
            DiagnosticKind::UndefinedNodeReference(name) if name == "Missing"
        )));
    }

    #[test]
    fn fan_in_sources_must_exist() {
        let src = "workflow F\n  start: S\n  exit: E\n  agent S\n    prompt: x\n    model: m\n    provider: p\n  fan_in J <- Ghost\n  agent E\n    prompt: y\n    model: m\n    provider: p\n  edges\n    S -> J\n    J -> E\n";
        let err = crate::parse(src, "t.dip").unwrap_err();
        assert!(err.diagnostics().iter().any(|d| matches!(
            &d.kind,
            DiagnosticKind::UndefinedNodeReference(name) if name == "Ghost"
        )));
    }
}
