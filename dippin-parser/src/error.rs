// ABOUTME: Structured error and diagnostic types for the dippin parser.
// ABOUTME: Replaces stringly-typed Result<_, String> across the public API.

use std::sync::Arc;

use crate::ir::SourceLocation;
use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error returned from `parse` and `convert_to_dot`.
#[derive(Debug, Clone, Error)]
#[non_exhaustive]
pub enum Error {
    /// One or more diagnostics were emitted while parsing.
    #[error("{} diagnostic(s) while parsing {file}", diagnostics.len())]
    Parse {
        file: Arc<str>,
        diagnostics: Vec<Diagnostic>,
    },
}

impl Error {
    /// Returns the diagnostics carried by this error.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        let Error::Parse { diagnostics, .. } = self;
        diagnostics
    }
}

/// A single diagnostic produced by the lexer or parser.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    pub severity: Severity,
    pub kind: DiagnosticKind,
    pub message: String,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Severity {
    Error,
    Warning,
}

/// Programmatic classification of a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiagnosticKind {
    UnexpectedToken { expected: String, found: String },
    UnterminatedString,
    UnknownCharacter(String),
    InvalidIndentation(String),
    InvalidInteger { value: String, field: String },
    InvalidFloat { value: String, field: String },
    InvalidDuration { value: String, field: String },
    InvalidBool { value: String, field: String },
    UnknownField { scope: String, name: String },
    MissingIdentifier { after: String },
    UndefinedNodeReference(String),
    DuplicateWorkflow,
    EmptyWorkflow,
    Other,
}

impl Diagnostic {
    pub fn error(
        kind: DiagnosticKind,
        message: impl Into<String>,
        location: SourceLocation,
    ) -> Self {
        Self {
            severity: Severity::Error,
            kind,
            message: message.into(),
            location,
        }
    }

    /// Render in `path:line:col: severity: message` form.
    pub fn render(&self) -> String {
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        format!(
            "{}:{}:{}: {}: {}",
            self.location.file, self.location.line, self.location.column, sev, self.message
        )
    }
}
