// ABOUTME: Public API for the dot-core library, exposed to Swift via UniFFI.
// ABOUTME: Provides DOT parsing, validation, and SVG rendering via Graphviz.

mod graphviz;
mod parser;

pub use parser::{DotGraph, DotStatement, SourceRange};

uniffi::setup_scaffolding!();

#[derive(uniffi::Enum)]
pub enum LayoutEngine {
    Dot,
    Neato,
    Fdp,
    Circo,
    Twopi,
    Sfdp,
}

#[derive(Debug, uniffi::Error)]
pub enum DotError {
    SyntaxError { message: String, line: u32, column: u32 },
    LayoutError { message: String },
    RenderError { message: String },
}

impl std::fmt::Display for DotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DotError::SyntaxError { message, line, column } => {
                write!(f, "Syntax error at {}:{}: {}", line, column, message)
            }
            DotError::LayoutError { message } => write!(f, "Layout error: {}", message),
            DotError::RenderError { message } => write!(f, "Render error: {}", message),
        }
    }
}

#[uniffi::export]
pub fn render_dot(dot_source: String, engine: LayoutEngine) -> Result<String, DotError> {
    graphviz::render_to_svg(&dot_source, &engine)
}

#[uniffi::export]
pub fn validate_dot(dot_source: String) -> Result<(), DotError> {
    graphviz::validate_syntax(&dot_source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_graph() {
        let dot = "digraph { a -> b }".to_string();
        let svg = render_dot(dot, LayoutEngine::Dot).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_render_invalid_dot() {
        let dot = "not a valid dot string {{{".to_string();
        let result = render_dot(dot, LayoutEngine::Dot);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_dot() {
        let dot = "digraph { a -> b }".to_string();
        assert!(validate_dot(dot).is_ok());
    }

    #[test]
    fn test_validate_invalid_dot() {
        let dot = "not valid {{{".to_string();
        assert!(validate_dot(dot).is_err());
    }
}
