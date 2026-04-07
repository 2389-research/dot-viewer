// ABOUTME: Indentation-aware tokenizer for the Dippin workflow language.
// ABOUTME: Produces a flat token stream with explicit INDENT/OUTDENT tokens.

use crate::error::{Diagnostic, DiagnosticKind};
use crate::ir::SourceLocation;

/// Token types produced by the lexer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    Error,
    Eof,
    Newline,
    Indent,
    Outdent,
    Keyword,
    Identifier,
    Operator,
    Literal,
    Colon,
    Comma,
    Arrow,
    BackArrow,
    LParen,
    RParen,
    RawBlock,
}

/// A single token with its type, value, and source location.
#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: String,
    pub location: SourceLocation,
}

/// Indentation-aware lexer for Dippin source files.
pub struct Lexer {
    lines: Vec<String>,
    line: usize,
    col: usize,
    indent_stack: Vec<usize>,
    tokens: Vec<Token>,
    token_idx: usize,
    filename: String,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

impl Lexer {
    /// Create a new Lexer and immediately tokenize the input.
    pub fn new(input: &str, filename: &str) -> Self {
        // Strip leading UTF-8 byte-order mark so editors that insert one don't
        // confuse the lexer (matches the Go reference parser).
        let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);
        // Normalize CRLF and lone CR line endings to LF so downstream
        // line-splitting works uniformly across platforms.
        let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<String> = normalized.split('\n').map(|s| s.to_string()).collect();
        let mut lexer = Lexer {
            lines,
            line: 1,
            col: 1,
            indent_stack: vec![0],
            tokens: Vec::new(),
            token_idx: 0,
            filename: filename.to_string(),
            diagnostics: Vec::new(),
        };
        lexer.lex();
        lexer
    }

    /// Consume and return the next token.
    pub fn next_token(&mut self) -> Token {
        if self.token_idx >= self.tokens.len() {
            return Token {
                token_type: TokenType::Eof,
                value: String::new(),
                location: SourceLocation {
                    file: self.filename.clone(),
                    line: self.line,
                    column: self.col,
                },
            };
        }
        let t = self.tokens[self.token_idx].clone();
        self.token_idx += 1;
        t
    }

    /// Peek at the next token without consuming it.
    pub fn peek_token(&self) -> Token {
        if self.token_idx >= self.tokens.len() {
            return Token {
                token_type: TokenType::Eof,
                value: String::new(),
                location: SourceLocation {
                    file: self.filename.clone(),
                    line: self.line,
                    column: self.col,
                },
            };
        }
        self.tokens[self.token_idx].clone()
    }

    /// Extract the raw value text from a line, starting after the colon.
    /// Used for single-line values like "fidelity: summary:medium".
    pub fn raw_value_text(&self, line_num: usize) -> String {
        if line_num < 1 || line_num > self.lines.len() {
            return String::new();
        }
        let line = &self.lines[line_num - 1];
        let line = line.trim_end();
        if let Some(colon_idx) = line.find(':') {
            let val = line[colon_idx + 1..].trim();
            maybe_strip_comment(val)
        } else {
            String::new()
        }
    }

    /// Main lexing loop: processes all lines.
    fn lex(&mut self) {
        let mut i = 0;
        while i < self.lines.len() {
            if let Some(new_i) = self.lex_one_line(i) {
                i = new_i;
            } else {
                i += 1;
            }
        }
        self.emit_remaining_outdents();
        self.tokens.push(Token {
            token_type: TokenType::Eof,
            value: String::new(),
            location: SourceLocation {
                file: self.filename.clone(),
                line: self.line,
                column: 1,
            },
        });
    }

    /// Process a single source line. Returns Some(new_i) if we consumed multiple
    /// lines (for raw blocks), or None to advance by one.
    fn lex_one_line(&mut self, i: usize) -> Option<usize> {
        let line = &self.lines[i].clone();
        self.line = i + 1;
        self.col = 1;
        let trimmed = line.trim_end();

        if is_blank_or_comment(trimmed) {
            return None;
        }

        let trimmed = strip_comment(trimmed);
        if trimmed.trim().is_empty() {
            return None;
        }

        self.check_indent_consistency(&trimmed, i + 1);
        let indent = line_indent(&trimmed);
        let content = &trimmed[indent..];

        self.emit_indent_tokens(indent);

        if is_key_colon_line(content) {
            return self.lex_key_colon_block(i, indent, content, line);
        }

        self.lex_line(content, indent);
        self.tokens.push(Token {
            token_type: TokenType::Newline,
            value: String::new(),
            location: SourceLocation {
                file: self.filename.clone(),
                line: self.line,
                column: line.chars().count() + 1,
            },
        });
        None
    }

    /// Diagnose lines whose leading whitespace mixes tabs and spaces. The
    /// language requires consistent indentation within a single line; mixing
    /// the two makes indent depth ambiguous across editors.
    fn check_indent_consistency(&mut self, line: &str, line_num: usize) {
        let leading: String = line.chars().take_while(|c| *c == ' ' || *c == '\t').collect();
        let has_tab = leading.contains('\t');
        let has_space = leading.contains(' ');
        if has_tab && has_space {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::InvalidIndentation("mixed tabs and spaces".into()),
                "indentation mixes tabs and spaces; use one or the other consistently",
                SourceLocation {
                    file: self.filename.clone(),
                    line: line_num,
                    column: 1,
                },
            ));
        }
    }

    /// Emit INDENT or OUTDENT tokens based on indentation change.
    fn emit_indent_tokens(&mut self, indent: usize) {
        let curr_indent = *self.indent_stack.last().unwrap();
        if indent > curr_indent {
            self.indent_stack.push(indent);
            self.tokens.push(Token {
                token_type: TokenType::Indent,
                value: String::new(),
                location: SourceLocation {
                    file: self.filename.clone(),
                    line: self.line,
                    column: 1,
                },
            });
            return;
        }
        while indent < *self.indent_stack.last().unwrap() && self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.tokens.push(Token {
                token_type: TokenType::Outdent,
                value: String::new(),
                location: SourceLocation {
                    file: self.filename.clone(),
                    line: self.line,
                    column: 1,
                },
            });
        }
        // After popping, the current indent must land exactly on a previously
        // pushed level; otherwise it's a dedent to an invented column.
        if indent != *self.indent_stack.last().unwrap() {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::InvalidIndentation(format!(
                    "dedent to column {} does not match any enclosing block",
                    indent
                )),
                "invalid dedent",
                SourceLocation {
                    file: self.filename.clone(),
                    line: self.line,
                    column: 1,
                },
            ));
        }
    }

    /// Emit remaining OUTDENT tokens at end of input.
    fn emit_remaining_outdents(&mut self) {
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.tokens.push(Token {
                token_type: TokenType::Outdent,
                value: String::new(),
                location: SourceLocation {
                    file: self.filename.clone(),
                    line: self.line,
                    column: 1,
                },
            });
        }
    }

    /// Handle a "key:" line, potentially followed by a multiline block.
    fn lex_key_colon_block(
        &mut self,
        i: usize,
        indent: usize,
        content: &str,
        line: &str,
    ) -> Option<usize> {
        let key_end = content.find(':').unwrap();
        let key = &content[..key_end];
        let loc = SourceLocation {
            file: self.filename.clone(),
            line: self.line,
            column: indent + 1,
        };
        self.tokens.push(Token {
            token_type: TokenType::Identifier,
            value: key.to_string(),
            location: loc,
        });
        self.tokens.push(Token {
            token_type: TokenType::Colon,
            value: ":".to_string(),
            location: SourceLocation {
                file: self.filename.clone(),
                line: self.line,
                column: indent + key_end + 1,
            },
        });

        if let Some(block_end) = self.try_collect_block(i, indent) {
            return Some(block_end);
        }

        // No multiline block follows — just a key: with empty value
        self.tokens.push(Token {
            token_type: TokenType::Newline,
            value: String::new(),
            location: SourceLocation {
                file: self.filename.clone(),
                line: self.line,
                column: line.chars().count() + 1,
            },
        });
        None
    }

    /// Look ahead for an indented block after a key: line.
    fn try_collect_block(&mut self, i: usize, indent: usize) -> Option<usize> {
        let next_content_line = self.find_next_content_line(i + 1);
        if next_content_line >= self.lines.len() {
            return None;
        }
        let next_indent = line_indent(&self.lines[next_content_line]);
        if next_indent <= indent {
            return None;
        }

        let block_end = self.find_block_end(next_content_line, indent);
        let raw_text = self.extract_raw_block(next_content_line, block_end, next_indent);
        self.tokens.push(Token {
            token_type: TokenType::Newline,
            value: String::new(),
            location: SourceLocation {
                file: self.filename.clone(),
                line: self.line,
                column: 1,
            },
        });
        self.tokens.push(Token {
            token_type: TokenType::RawBlock,
            value: raw_text,
            location: SourceLocation {
                file: self.filename.clone(),
                line: next_content_line + 1,
                column: next_indent + 1,
            },
        });
        self.tokens.push(Token {
            token_type: TokenType::Newline,
            value: String::new(),
            location: SourceLocation {
                file: self.filename.clone(),
                line: block_end + 1,
                column: 1,
            },
        });
        Some(block_end)
    }

    /// Find the next non-blank, non-comment line starting from idx.
    fn find_next_content_line(&self, mut idx: usize) -> usize {
        while idx < self.lines.len() {
            let trimmed = self.lines[idx].trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                return idx;
            }
            idx += 1;
        }
        idx
    }

    /// Find the end of a multiline block (where indentation drops back).
    fn find_block_end(&self, start: usize, indent: usize) -> usize {
        let mut block_end = start;
        while block_end < self.lines.len() {
            let bl = self.lines[block_end].trim_end();
            if bl.trim().is_empty() {
                block_end += 1;
                continue;
            }
            if line_indent(bl) <= indent {
                break;
            }
            block_end += 1;
        }
        block_end
    }

    /// Extract raw text from original lines, stripping indent prefix.
    fn extract_raw_block(&self, start_idx: usize, end_idx: usize, indent: usize) -> String {
        let mut result: Vec<String> = Vec::new();
        for i in start_idx..end_idx.min(self.lines.len()) {
            result.push(strip_indent_prefix(&self.lines[i], indent));
        }
        trim_trailing_blank_lines(&result)
    }

    /// Tokenize a single line of content (after indent has been handled).
    /// Columns are reported as 1-based character offsets so that multi-byte
    /// UTF-8 sequences don't inflate the reported column. We maintain a
    /// running `char_col` cursor alongside the byte cursor `i` so the char
    /// position is computed in O(total chars) rather than O(tokens * bytes).
    ///
    /// `indent` is the column (1-based, char offset) of `line[0]` within the
    /// original source line minus one — i.e. the number of leading whitespace
    /// characters stripped before `content` was sliced. Passing it explicitly
    /// keeps `lex_line` from implicitly depending on the indent stack.
    fn lex_line(&mut self, line: &str, indent: usize) {
        let mut i = 0;
        // `col_offset` is the 1-based char column of `line[0]` in the original
        // source line. The indent prefix is pure ASCII (space/tab), so its
        // char count equals its byte count.
        let col_offset = 1 + indent;
        let mut char_col = col_offset;

        while i < line.len() {
            let ws_start = i;
            i = skip_whitespace(line, i);
            // Leading whitespace is ASCII (space/tab), so byte count == char count.
            char_col += i - ws_start;
            if i >= line.len() {
                break;
            }
            let loc = SourceLocation {
                file: self.filename.clone(),
                line: self.line,
                column: char_col,
            };
            let new_i = self.lex_one_token(line, i, loc);
            // Advance the char cursor by the true char width of the bytes
            // the token consumed (quoted strings may hold multi-byte chars).
            char_col += line[i..new_i].chars().count();
            i = new_i;
        }
    }

    /// Try each token type and return the new position.
    fn lex_one_token(&mut self, line: &str, i: usize, loc: SourceLocation) -> usize {
        if let Some(new_i) = self.try_lex_punctuation(line, i, &loc) {
            return new_i;
        }
        if let Some(new_i) = self.try_lex_arrow(line, i, &loc) {
            return new_i;
        }
        if let Some(new_i) = self.try_lex_operator(line, i, &loc) {
            return new_i;
        }
        if let Some(new_i) = self.try_lex_quoted_string(line, i, &loc) {
            return new_i;
        }
        if let Some(new_i) = self.try_lex_identifier(line, i, &loc) {
            return new_i;
        }
        // Unknown character: emit diagnostic and advance past the full UTF-8 code point
        let ch = line[i..].chars().next();
        let ch_str = ch.map(|c| c.to_string()).unwrap_or_default();
        self.diagnostics.push(Diagnostic::error(
            DiagnosticKind::UnknownCharacter(ch_str.clone()),
            format!("unknown character {:?}", ch_str),
            loc,
        ));
        i + ch.map_or(1, |c| c.len_utf8())
    }

    /// Handle single-character punctuation: : , ( )
    fn try_lex_punctuation(
        &mut self,
        line: &str,
        i: usize,
        loc: &SourceLocation,
    ) -> Option<usize> {
        let ch = line.as_bytes()[i];
        // Map directly to the static string form so we don't round-trip a
        // byte through `char` (which would silently mishandle non-ASCII).
        let (tok_type, value): (TokenType, &'static str) = match ch {
            b':' => (TokenType::Colon, ":"),
            b',' => (TokenType::Comma, ","),
            b'(' => (TokenType::LParen, "("),
            b')' => (TokenType::RParen, ")"),
            _ => return None,
        };
        self.tokens.push(Token {
            token_type: tok_type,
            value: value.to_string(),
            location: loc.clone(),
        });
        Some(i + 1)
    }

    /// Handle two-character arrows: -> and <-
    fn try_lex_arrow(&mut self, line: &str, i: usize, loc: &SourceLocation) -> Option<usize> {
        if line[i..].starts_with("->") {
            self.tokens.push(Token {
                token_type: TokenType::Arrow,
                value: "->".to_string(),
                location: loc.clone(),
            });
            return Some(i + 2);
        }
        if line[i..].starts_with("<-") {
            self.tokens.push(Token {
                token_type: TokenType::BackArrow,
                value: "<-".to_string(),
                location: loc.clone(),
            });
            return Some(i + 2);
        }
        None
    }

    /// Handle comparison operators: ==, !=, <=, >=, =, <, >, !
    fn try_lex_operator(&mut self, line: &str, i: usize, loc: &SourceLocation) -> Option<usize> {
        let ch = line.as_bytes()[i];
        if !is_operator_char(ch) {
            return None;
        }
        if i + 1 < line.len() {
            let two = &line[i..i + 2];
            if two == "==" || two == "!=" || two == "<=" || two == ">=" {
                self.tokens.push(Token {
                    token_type: TokenType::Operator,
                    value: two.to_string(),
                    location: loc.clone(),
                });
                return Some(i + 2);
            }
        }
        let value: &'static str = match ch {
            b'=' => "=",
            b'!' => "!",
            b'<' => "<",
            b'>' => ">",
            _ => return None,
        };
        self.tokens.push(Token {
            token_type: TokenType::Operator,
            value: value.to_string(),
            location: loc.clone(),
        });
        Some(i + 1)
    }

    /// Handle double-quoted string literals with escape sequences.
    fn try_lex_quoted_string(
        &mut self,
        line: &str,
        i: usize,
        loc: &SourceLocation,
    ) -> Option<usize> {
        if line.as_bytes()[i] != b'"' {
            return None;
        }
        let (content, new_i, terminated) = read_quoted_content(line, i + 1);
        if !terminated {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::UnterminatedString,
                "unterminated string literal",
                loc.clone(),
            ));
        }
        self.tokens.push(Token {
            token_type: TokenType::Literal,
            value: content,
            location: loc.clone(),
        });
        Some(new_i)
    }

    /// Handle alphanumeric identifiers including _, -, ., /
    fn try_lex_identifier(
        &mut self,
        line: &str,
        i: usize,
        loc: &SourceLocation,
    ) -> Option<usize> {
        let bytes = line.as_bytes();
        if !is_alpha_num(bytes[i]) {
            return None;
        }
        let start = i;
        let mut j = i;
        while j < bytes.len() && is_ident_word_char(bytes[j]) {
            j += 1;
        }
        self.tokens.push(Token {
            token_type: TokenType::Identifier,
            value: line[start..j].to_string(),
            location: loc.clone(),
        });
        Some(j)
    }
}

/// Return the number of leading whitespace bytes in a line.
fn line_indent(line: &str) -> usize {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    i
}

/// Check if a line is empty, whitespace-only, or a comment-only line.
fn is_blank_or_comment(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

/// Find an unquoted `#` character in a string, returning the byte offset.
/// Iterates by `char` rather than byte so that escaped multi-byte characters
/// (e.g. `\é`) advance by the correct width.
fn find_unquoted_hash(line: &str) -> Option<usize> {
    let mut chars = line.char_indices();
    let mut in_quote = false;
    while let Some((i, ch)) = chars.next() {
        if in_quote {
            if ch == '\\' {
                // Consume the escaped char regardless of its UTF-8 width.
                chars.next();
            } else if ch == '"' {
                in_quote = false;
            }
        } else if ch == '"' {
            in_quote = true;
        } else if ch == '#' {
            return Some(i);
        }
    }
    None
}

/// Strip a trailing comment from a line.
fn strip_comment(line: &str) -> String {
    let trimmed = line.trim_end();
    if let Some(idx) = find_unquoted_hash(trimmed) {
        if is_strippable_hash(trimmed, idx) {
            return trimmed[..idx].to_string();
        }
    }
    trimmed.to_string()
}

/// Check if the # at idx should be treated as a comment start.
fn is_strippable_hash(line: &str, idx: usize) -> bool {
    if idx == line_indent(line) {
        return true;
    }
    let bytes = line.as_bytes();
    idx > 0 && (bytes[idx - 1] == b' ' || bytes[idx - 1] == b'\t')
}

/// Check if a line content (after indent) is just "identifier:" with nothing after.
fn is_key_colon_line(content: &str) -> bool {
    if let Some(colon_idx) = content.find(':') {
        if colon_idx == 0 {
            return false;
        }
        let key = &content[..colon_idx];
        if !key.bytes().all(is_ident_rune) {
            return false;
        }
        let after = content[colon_idx + 1..].trim();
        after.is_empty()
    } else {
        false
    }
}

/// Check if a byte is valid in an identifier (alphanumeric or underscore).
fn is_ident_rune(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || ch == b'_'
}

/// Strip up to `indent` whitespace bytes from the front of a line.
fn strip_indent_prefix(line: &str, indent: usize) -> String {
    let line = line.trim_end_matches('\r');
    let bytes = line.as_bytes();
    let mut j = 0;
    while j < bytes.len() && j < indent && (bytes[j] == b' ' || bytes[j] == b'\t') {
        j += 1;
    }
    line[j..].to_string()
}

/// Remove trailing blank lines from a Vec and join.
fn trim_trailing_blank_lines(lines: &[String]) -> String {
    let mut end = lines.len();
    while end > 0 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    lines[..end].join("\n")
}

/// Skip whitespace characters.
fn skip_whitespace(line: &str, mut i: usize) -> usize {
    let bytes = line.as_bytes();
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    i
}

/// Check if a byte is an operator start character.
fn is_operator_char(ch: u8) -> bool {
    ch == b'=' || ch == b'!' || ch == b'<' || ch == b'>'
}

/// Read characters from line[start:] until an unescaped closing quote.
/// Uses char iteration to correctly handle multi-byte UTF-8 sequences.
/// Returns (content, new_index, terminated) where terminated is false if EOL hit.
fn read_quoted_content(line: &str, start: usize) -> (String, usize, bool) {
    let mut content = String::new();
    let mut chars = line[start..].char_indices();
    while let Some((offset, ch)) = chars.next() {
        if ch == '"' {
            return (content, start + offset + 1, true);
        }
        if ch == '\\' {
            if let Some((_next_offset, escaped_ch)) = chars.next() {
                content.push(escaped_ch);
            }
        } else {
            content.push(ch);
        }
    }
    // Reached end of line without closing quote
    (content, line.len(), false)
}

/// Check if a byte is alphanumeric.
fn is_alpha_num(ch: u8) -> bool {
    ch.is_ascii_alphanumeric()
}

/// Check if a byte is valid within an identifier word (alphanumeric, _, -, ., /).
fn is_ident_word_char(ch: u8) -> bool {
    is_alpha_num(ch) || ch == b'_' || ch == b'-' || ch == b'.' || ch == b'/'
}

/// Strip an inline comment from val unless val starts with # or ".
fn maybe_strip_comment(val: &str) -> String {
    if val.is_empty() || val.starts_with('#') || val.starts_with('"') {
        return val.to_string();
    }
    strip_comment(val).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_indent() {
        assert_eq!(line_indent("hello"), 0);
        assert_eq!(line_indent("  hello"), 2);
        assert_eq!(line_indent("    hello"), 4);
        assert_eq!(line_indent("\thello"), 1);
    }

    #[test]
    fn test_is_blank_or_comment() {
        assert!(is_blank_or_comment(""));
        assert!(is_blank_or_comment("   "));
        assert!(is_blank_or_comment("# comment"));
        assert!(is_blank_or_comment("  # comment"));
        assert!(!is_blank_or_comment("hello"));
    }

    #[test]
    fn test_is_key_colon_line() {
        assert!(is_key_colon_line("prompt:"));
        assert!(is_key_colon_line("command:"));
        assert!(!is_key_colon_line("prompt: hello"));
        assert!(!is_key_colon_line("hello"));
        assert!(!is_key_colon_line(":"));
    }

    #[test]
    fn test_strip_comment() {
        assert_eq!(strip_comment("hello # comment"), "hello ");
        assert_eq!(strip_comment("hello"), "hello");
        assert_eq!(strip_comment("# comment"), "");
    }

    #[test]
    fn test_basic_tokenization() {
        let input = "workflow Test\n  goal: test\n";
        let lexer = Lexer::new(input, "test.dip");
        // Should produce: Identifier("workflow"), Identifier("Test"), Newline,
        // Indent, Identifier("goal"), Colon, Identifier("test"), Newline, Outdent, EOF
        let tokens: Vec<_> = lexer.tokens.iter().map(|t| &t.token_type).collect();
        assert!(tokens.contains(&&TokenType::Identifier));
        assert!(tokens.contains(&&TokenType::Indent));
        assert!(tokens.contains(&&TokenType::Outdent));
        assert!(tokens.contains(&&TokenType::Eof));
    }

    #[test]
    fn test_arrow_tokenization() {
        let input = "Start -> End\n";
        let lexer = Lexer::new(input, "test.dip");
        let token_types: Vec<_> = lexer.tokens.iter().map(|t| &t.token_type).collect();
        assert!(token_types.contains(&&TokenType::Arrow));
    }

    #[test]
    fn test_back_arrow_tokenization() {
        let input = "Join <- A, B\n";
        let lexer = Lexer::new(input, "test.dip");
        let token_types: Vec<_> = lexer.tokens.iter().map(|t| &t.token_type).collect();
        assert!(token_types.contains(&&TokenType::BackArrow));
        assert!(token_types.contains(&&TokenType::Comma));
    }

    #[test]
    fn test_raw_block_collection() {
        let input = "prompt:\n  Hello world\n  Second line\n";
        let lexer = Lexer::new(input, "test.dip");
        let raw_blocks: Vec<_> = lexer
            .tokens
            .iter()
            .filter(|t| t.token_type == TokenType::RawBlock)
            .collect();
        assert_eq!(raw_blocks.len(), 1);
        assert_eq!(raw_blocks[0].value, "Hello world\nSecond line");
    }

    #[test]
    fn test_quoted_string() {
        let input = "label: \"Hello World\"\n";
        let lexer = Lexer::new(input, "test.dip");
        // Find the literal token
        let literal = lexer
            .tokens
            .iter()
            .find(|t| t.token_type == TokenType::Literal)
            .unwrap();
        assert_eq!(literal.value, "Hello World");
    }

    #[test]
    fn test_operator_tokenization() {
        let input = "ctx.outcome == success\n";
        let lexer = Lexer::new(input, "test.dip");
        let ops: Vec<_> = lexer
            .tokens
            .iter()
            .filter(|t| t.token_type == TokenType::Operator)
            .collect();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].value, "==");
    }

    #[test]
    fn test_lexer_strips_bom() {
        let src = "\u{FEFF}workflow Foo\n";
        let mut lex = Lexer::new(src, "test.dip");
        let tok = lex.next_token();
        assert_eq!(tok.token_type, TokenType::Identifier);
        assert_eq!(tok.value, "workflow");
        assert!(
            lex.diagnostics.is_empty(),
            "BOM should be stripped silently, got {:?}",
            lex.diagnostics
        );
    }

    #[test]
    fn test_lexer_handles_crlf() {
        let src = "workflow Foo\r\n  start: A\r\n  exit: A\r\nagent A\r\n  prompt: \"x\"\r\n  model: m\r\n  provider: p\r\n";
        let wf = crate::parse(src, "test.dip").expect("CRLF should parse");
        assert_eq!(wf.name, "Foo");
    }

    #[test]
    fn test_lexer_handles_cr_only() {
        let src = "workflow Foo\r  start: A\r  exit: A\ragent A\r  prompt: \"x\"\r  model: m\r  provider: p\r";
        let wf = crate::parse(src, "test.dip").expect("CR-only should parse");
        assert_eq!(wf.name, "Foo");
    }

    #[test]
    fn test_lexer_rejects_mixed_indent() {
        // tab then 2 spaces — clearly mixed
        let src = "workflow Foo\n\t  goal: bar\n";
        let err = crate::parse(src, "test.dip").unwrap_err();
        assert!(
            err.diagnostics()
                .iter()
                .any(|d| matches!(d.kind, crate::DiagnosticKind::InvalidIndentation(_))),
            "expected InvalidIndentation diagnostic, got {:?}",
            err.diagnostics()
        );
    }

    #[test]
    fn test_lexer_rejects_invalid_dedent() {
        // dedent to a level that was never pushed
        let src = "workflow Foo\n    goal: bar\n  exit: x\n";
        let err = crate::parse(src, "test.dip").unwrap_err();
        assert!(
            err.diagnostics()
                .iter()
                .any(|d| matches!(d.kind, crate::DiagnosticKind::InvalidIndentation(_))),
            "expected InvalidIndentation diagnostic, got {:?}",
            err.diagnostics()
        );
    }

    #[test]
    fn test_lexer_columns_are_char_offsets() {
        // 'é' is 2 bytes in UTF-8; tokens after a multi-byte char inside a
        // quoted string must still report their column in characters.
        let src = "label \"é\" bar\n";
        let mut lex = Lexer::new(src, "t.dip");
        let t1 = lex.next_token();
        assert_eq!(t1.value, "label");
        assert_eq!(t1.location.column, 1);
        let t2 = lex.next_token();
        assert_eq!(t2.token_type, TokenType::Literal);
        assert_eq!(t2.location.column, 7);
        let t3 = lex.next_token();
        assert_eq!(t3.value, "bar");
        assert_eq!(
            t3.location.column, 11,
            "bar should be at char column 11, got {}",
            t3.location.column
        );
    }

    #[test]
    fn test_lexer_columns_after_multiple_multibyte_tokens() {
        // Two quoted strings each containing multi-byte chars; the trailing
        // identifier must be reported at the correct char column even though
        // the earlier tokens span more bytes than chars.
        let src = "label \"é\" note \"ü\" tail\n";
        // char layout (1-based):
        //  1 l
        //  2 a
        //  3 b
        //  4 e
        //  5 l
        //  6 space
        //  7 "
        //  8 é
        //  9 "
        // 10 space
        // 11 n
        // 12 o
        // 13 t
        // 14 e
        // 15 space
        // 16 "
        // 17 ü
        // 18 "
        // 19 space
        // 20 t (tail starts here)
        let mut lex = Lexer::new(src, "t.dip");
        let mut saw_tail = false;
        loop {
            let t = lex.next_token();
            if t.token_type == TokenType::Eof {
                break;
            }
            if t.value == "tail" {
                assert_eq!(
                    t.location.column, 20,
                    "tail should be at char column 20, got {}",
                    t.location.column
                );
                saw_tail = true;
            }
        }
        assert!(saw_tail, "did not encounter tail token");
    }

    #[test]
    fn test_find_unquoted_hash_utf8_safe() {
        // backslash followed by multi-byte char inside a string, then a # outside
        let src = r#"label "a\é" # comment"#;
        let result = find_unquoted_hash(src);
        let expected = src.find("# comment").unwrap();
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_raw_value_text() {
        let input = "fidelity: summary:medium\nmodel: gpt-4\n";
        let lexer = Lexer::new(input, "test.dip");
        assert_eq!(lexer.raw_value_text(1), "summary:medium");
        assert_eq!(lexer.raw_value_text(2), "gpt-4");
    }
}
