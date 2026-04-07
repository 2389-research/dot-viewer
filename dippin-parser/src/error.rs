// ABOUTME: Structured error and diagnostic types for the dippin parser.
// ABOUTME: Replaces stringly-typed Result<_, String> across the public API.

use std::sync::Arc;

use crate::ir::SourceLocation;
use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error returned from `parse` and `parse_to_dot`.
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

// Manual serde impls because the `Parse` variant carries `Arc<str>`, which does
// not implement `Deserialize` without serde's `rc` feature; we round-trip the
// file path as a `String`.
#[cfg(feature = "serde")]
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStructVariant;
        let Error::Parse { file, diagnostics } = self;
        let mut sv = serializer.serialize_struct_variant("Error", 0, "Parse", 2)?;
        sv.serialize_field("file", &**file)?;
        sv.serialize_field("diagnostics", diagnostics)?;
        sv.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Error {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        enum Helper {
            Parse {
                file: String,
                diagnostics: Vec<Diagnostic>,
            },
        }
        let h = Helper::deserialize(deserializer)?;
        let Helper::Parse { file, diagnostics } = h;
        Ok(Error::Parse {
            file: Arc::from(file),
            diagnostics,
        })
    }
}

impl Error {
    /// Returns the diagnostics carried by this error.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        let Error::Parse { diagnostics, .. } = self;
        diagnostics
    }
}

/// A single diagnostic produced by the lexer or parser.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Diagnostic {
    pub severity: Severity,
    pub kind: DiagnosticKind,
    pub message: String,
    pub location: SourceLocation,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Severity {
    Error,
    Warning,
}

/// Programmatic classification of a diagnostic.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
