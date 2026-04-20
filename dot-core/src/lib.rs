// ABOUTME: Public API for the dot-core library, exposed to Swift via UniFFI.
// ABOUTME: Provides DOT parsing, validation, and rendering (SVG, plain) via Graphviz.

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
#[uniffi::export]
pub fn render_dot(dot_source: String, engine: LayoutEngine) -> Result<String, DotError> {
    graphviz::render_to_svg(&dot_source, &engine)
}

#[cfg(not(target_arch = "wasm32"))]
#[uniffi::export]
pub fn render_dot_plain(dot_source: String, engine: LayoutEngine) -> Result<String, DotError> {
    graphviz::render_to_plain(&dot_source, &engine)
}

#[cfg(not(target_arch = "wasm32"))]
#[uniffi::export]
pub fn validate_dot(dot_source: String) -> Result<(), DotError> {
    graphviz::validate_syntax(&dot_source)
}

/// A pair of byte ranges linking a fragment of generated DOT to its original
/// dippin source location.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, uniffi::Record)]
pub struct SourceMapEntry {
    pub dot_start: u32,
    pub dot_end: u32,
    pub dip_start: u32,
    pub dip_end: u32,
}

/// Result of converting dippin to DOT via [`parse_dippin`].
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, uniffi::Record)]
pub struct DippinConversionResult {
    pub dot_source: String,
    pub source_map: Vec<SourceMapEntry>,
}

/// Parse a dippin source string and return the converted DOT plus a source
/// map connecting each node/edge back to its dippin origin.
#[cfg(not(target_arch = "wasm32"))]
#[uniffi::export]
pub fn parse_dippin(source: String) -> Result<DippinConversionResult, DotError> {
    let conv = dippin_parser::parse_to_dot_with_map(&source, "input.dip").map_err(|e| {
        let (message, line, column) = e
            .diagnostics()
            .first()
            .map(|d| {
                (
                    format!(
                        "{}:{}:{}: {}",
                        d.location.file, d.location.line, d.location.column, d.message
                    ),
                    d.location.line as u32,
                    d.location.column as u32,
                )
            })
            .unwrap_or_else(|| ("dippin parse failed".to_string(), 0, 0));
        DotError::SyntaxError { message, line, column }
    })?;
    let source_map = conv
        .source_map
        .into_iter()
        .map(|e| SourceMapEntry {
            dot_start: e.dot_range.start as u32,
            dot_end: e.dot_range.end as u32,
            dip_start: e.dip_range.start as u32,
            dip_end: e.dip_range.end as u32,
        })
        .collect();
    Ok(DippinConversionResult {
        dot_source: conv.dot_source,
        source_map,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_render_simple_graph() {
        let dot = "digraph { a -> b }".to_string();
        let svg = render_dot(dot, LayoutEngine::Dot).unwrap();
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_render_invalid_dot() {
        let dot = "not a valid dot string {{{".to_string();
        let result = render_dot(dot, LayoutEngine::Dot);
        assert!(result.is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_validate_valid_dot() {
        let dot = "digraph { a -> b }".to_string();
        assert!(validate_dot(dot).is_ok());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_validate_invalid_dot() {
        let dot = "not valid {{{".to_string();
        assert!(validate_dot(dot).is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_render_plain_format() {
        let dot = "digraph { a -> b }".to_string();
        let plain = render_dot_plain(dot, LayoutEngine::Dot).unwrap();
        assert!(plain.starts_with("graph"));
        assert!(plain.contains("node a"));
        assert!(plain.contains("node b"));
        assert!(plain.contains("edge a b"));
        assert!(plain.contains("stop"));
    }
}
