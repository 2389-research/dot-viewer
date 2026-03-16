// ABOUTME: Parser for Graphviz's "plain" text output format.
// ABOUTME: Converts plain format lines into structured PlainGraph with nodes and edges.

/// A node from Graphviz plain format output.
pub struct PlainNode {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub label: String,
}

/// An edge from Graphviz plain format output.
pub struct PlainEdge {
    pub from: String,
    pub to: String,
    pub points: Vec<(f64, f64)>,
    pub label: Option<String>,
}

/// A parsed Graphviz plain format graph with dimensions, nodes, and edges.
pub struct PlainGraph {
    pub width: f64,
    pub height: f64,
    pub nodes: Vec<PlainNode>,
    pub edges: Vec<PlainEdge>,
}

/// Known edge style keywords used to detect absence of a label.
const STYLE_KEYWORDS: &[&str] = &["solid", "dashed", "dotted", "bold", "invis"];

/// Tokenize a line, respecting double-quoted strings.
fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        if ch == '"' {
            chars.next(); // consume opening quote
            let mut token = String::new();
            while let Some(&c) = chars.peek() {
                if c == '"' {
                    chars.next(); // consume closing quote
                    break;
                }
                // Handle escaped characters inside quotes
                if c == '\\' {
                    chars.next();
                    if let Some(&escaped) = chars.peek() {
                        token.push(escaped);
                        chars.next();
                        continue;
                    }
                }
                token.push(c);
                chars.next();
            }
            tokens.push(token);
        } else {
            let mut token = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                token.push(c);
                chars.next();
            }
            tokens.push(token);
        }
    }

    tokens
}

/// Parse a float token, returning a descriptive error on failure.
fn parse_f64(token: &str, context: &str) -> Result<f64, String> {
    token
        .parse::<f64>()
        .map_err(|e| format!("invalid {} '{}': {}", context, token, e))
}

/// Parse Graphviz plain format text into a structured PlainGraph.
///
/// The plain format consists of lines starting with:
/// - `graph <scale> <width> <height>`
/// - `node <name> <x> <y> <width> <height> <label> <style> <shape> <color> <fillcolor>`
/// - `edge <from> <to> <n> <x1> <y1> ... <xn> <yn> [label] <style> <color>`
/// - `stop`
///
/// All coordinates are multiplied by the graph scale factor.
pub fn parse_plain(input: &str) -> Result<PlainGraph, String> {
    let mut scale = 1.0;
    let mut width = 0.0;
    let mut height = 0.0;
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let tokens = tokenize(line);
        if tokens.is_empty() {
            continue;
        }

        match tokens[0].as_str() {
            "graph" => {
                if tokens.len() < 4 {
                    return Err("graph line requires scale, width, height".to_string());
                }
                scale = parse_f64(&tokens[1], "graph scale")?;
                width = parse_f64(&tokens[2], "graph width")? * scale;
                height = parse_f64(&tokens[3], "graph height")? * scale;
            }
            "node" => {
                if tokens.len() < 7 {
                    return Err("node line requires name, x, y, width, height, label".to_string());
                }
                let name = tokens[1].clone();
                let x = parse_f64(&tokens[2], "node x")? * scale;
                let y = parse_f64(&tokens[3], "node y")? * scale;
                let w = parse_f64(&tokens[4], "node width")? * scale;
                let h = parse_f64(&tokens[5], "node height")? * scale;
                let label = tokens[6].clone();
                nodes.push(PlainNode {
                    name,
                    x,
                    y,
                    width: w,
                    height: h,
                    label,
                });
            }
            "edge" => {
                if tokens.len() < 4 {
                    return Err("edge line requires from, to, point count".to_string());
                }
                let from = tokens[1].clone();
                let to = tokens[2].clone();
                let n = tokens[3]
                    .parse::<usize>()
                    .map_err(|e| format!("invalid edge point count '{}': {}", tokens[3], e))?;

                let coords_start = 4;
                let coords_end = coords_start + n * 2;
                if tokens.len() < coords_end {
                    return Err(format!(
                        "edge line claims {} points but not enough tokens",
                        n
                    ));
                }

                let mut points = Vec::with_capacity(n);
                for i in 0..n {
                    let xi = parse_f64(&tokens[coords_start + i * 2], "edge x")? * scale;
                    let yi = parse_f64(&tokens[coords_start + i * 2 + 1], "edge y")? * scale;
                    points.push((xi, yi));
                }

                // After coordinates: optionally a label, then style and color.
                // If the next token is a known style keyword, there's no label.
                let label = if tokens.len() > coords_end {
                    let candidate = &tokens[coords_end];
                    if STYLE_KEYWORDS.contains(&candidate.as_str()) {
                        None
                    } else {
                        Some(candidate.clone())
                    }
                } else {
                    None
                };

                edges.push(PlainEdge {
                    from,
                    to,
                    points,
                    label,
                });
            }
            "stop" => break,
            other => {
                return Err(format!("unknown line type '{}'", other));
            }
        }
    }

    Ok(PlainGraph {
        width,
        height,
        nodes,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
graph 1 2.75 4.5
node a 1.375 4.0694 0.75 0.5 a solid ellipse black lightgrey
node b 1.375 3.0694 0.75 0.5 b solid ellipse black lightgrey
edge a b 4 1.375 3.8195 1.375 3.7114 1.375 3.5813 1.375 3.4612 solid black
stop
";

    #[test]
    fn test_parse_plain_nodes() {
        let graph = parse_plain(SAMPLE).unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.nodes[0].name, "a");
        assert_eq!(graph.nodes[1].name, "b");
    }

    #[test]
    fn test_parse_plain_edges() {
        let graph = parse_plain(SAMPLE).unwrap();
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.edges[0].from, "a");
        assert_eq!(graph.edges[0].to, "b");
        assert_eq!(graph.edges[0].points.len(), 4);
    }

    #[test]
    fn test_parse_plain_dimensions() {
        let graph = parse_plain(SAMPLE).unwrap();
        assert!((graph.width - 2.75).abs() < 1e-6);
        assert!((graph.height - 4.5).abs() < 1e-6);
    }

    #[test]
    fn test_parse_plain_with_labels() {
        let input = "\
graph 1 3.0 5.0
node hello 1.0 2.0 0.75 0.5 \"Hello World\" solid ellipse black lightgrey
node bye 1.0 4.0 0.75 0.5 \"Goodbye\" solid ellipse black lightgrey
edge hello bye 2 1.0 2.5 1.0 3.5 \"my label\" solid black
stop
";
        let graph = parse_plain(input).unwrap();
        assert_eq!(graph.nodes[0].label, "Hello World");
        assert_eq!(graph.nodes[1].label, "Goodbye");
        assert_eq!(graph.edges[0].label.as_deref(), Some("my label"));
    }

    #[test]
    fn test_parse_plain_scale_factor() {
        let input = "\
graph 2 3.0 4.0
node a 1.0 2.0 0.5 0.5 a solid ellipse black lightgrey
stop
";
        let graph = parse_plain(input).unwrap();
        // All coordinates multiplied by scale=2
        assert!((graph.width - 6.0).abs() < 1e-6);
        assert!((graph.height - 8.0).abs() < 1e-6);
        assert!((graph.nodes[0].x - 2.0).abs() < 1e-6);
        assert!((graph.nodes[0].y - 4.0).abs() < 1e-6);
    }
}
