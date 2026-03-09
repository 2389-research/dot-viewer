// ABOUTME: Public API for the dot-core library, exposed to Swift via UniFFI.
// ABOUTME: Provides DOT parsing, validation, and SVG rendering via Graphviz.

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
pub fn render_dot(_dot_source: String, _engine: LayoutEngine) -> Result<String, DotError> {
    Err(DotError::RenderError { message: "not yet implemented".to_string() })
}

#[uniffi::export]
pub fn validate_dot(_dot_source: String) -> Result<(), DotError> {
    Err(DotError::SyntaxError { message: "not yet implemented".to_string(), line: 0, column: 0 })
}
