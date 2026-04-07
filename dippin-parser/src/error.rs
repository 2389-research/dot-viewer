// ABOUTME: Structured error and diagnostic types for the dippin parser.
// ABOUTME: Replaces stringly-typed Result<_, String> across the public API.

use crate::ir::SourceLocation;
use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error returned from `parse` and `convert_to_dot`.
#[derive(Debug, Clone, Error)]
pub enum Error {
    /// One or more diagnostics were emitted while parsing.
    #[error("{} diagnostic(s) while parsing {file}", diagnostics.len())]
    Parse {
        file: String,
        diagnostics: Vec<Diagnostic>,
    },
    /// I/O error reading a file.
    #[error("I/O error: {0}")]
    Io(String),
}

impl Error {
    /// Returns the diagnostics if this is a `Parse` error.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            Error::Parse { diagnostics, .. } => diagnostics,
            _ => &[],
        }
    }
}

/// A single diagnostic produced by the lexer or parser.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub kind: DiagnosticKind,
    pub message: String,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
}

/// Programmatic classification of a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
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
