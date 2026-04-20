# Dippin Parser Correctness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Harden `dippin-parser` against malformed input, fix lexer compatibility bugs, achieve behavioral parity with the upstream Go reference, and replace stringly-typed errors with structured diagnostics.

**Architecture:** Introduce a typed `Error`/`Diagnostic` system with preserved `SourceLocation`. Make the parser fail-fast on structural errors while still accumulating multiple diagnostics. Normalize input encoding (CRLF/CR/BOM) at the lexer boundary. Port behavior from `/Users/dylanr/work/2389/dippin-lang/` where the Rust port silently diverges.

**Tech Stack:** Rust 2021, `thiserror`, `dippin-parser` workspace crate, `dot-viewer-cli` for end-user diagnostics. Reference impl: Go at `/Users/dylanr/work/2389/dippin-lang/parser/`.

**Companion plans:**
- `docs/plans/2026-04-07-dippin-api-polish.md` (API hardening + docs) — run AFTER this
- `docs/plans/2026-04-07-dippin-ux-and-tests.md` (CLI UX + test coverage) — run AFTER this

---

## Phase 1: Typed errors & diagnostics

### Task 1: Add `thiserror` dependency

**Files:**
- Modify: `dippin-parser/Cargo.toml`

**Step 1: Add dependency**

In `[dependencies]` add:
```toml
thiserror = "1"
```

**Step 2: Verify build**

```bash
cargo build -p dippin-parser
```
Expected: clean build.

**Step 3: Commit**

```bash
git add dippin-parser/Cargo.toml dippin-parser/Cargo.lock 2>/dev/null || git add dippin-parser/Cargo.toml Cargo.lock
git commit -m "deps: add thiserror to dippin-parser"
```

---

### Task 2: Define `Error` and `Diagnostic` types

**Files:**
- Create: `dippin-parser/src/error.rs`
- Modify: `dippin-parser/src/lib.rs`

**Step 1: Create the error module**

Create `dippin-parser/src/error.rs`:

```rust
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
```

**Step 2: Wire module into lib.rs**

In `dippin-parser/src/lib.rs`, add near the top of the module declarations:
```rust
pub mod error;
```
And add to the public re-exports:
```rust
pub use error::{Diagnostic, DiagnosticKind, Error, Result, Severity};
```

**Step 3: Build**

```bash
cargo build -p dippin-parser
```
Expected: clean build.

**Step 4: Commit**

```bash
git add dippin-parser/src/error.rs dippin-parser/src/lib.rs
git commit -m "feat(parser): add structured Error and Diagnostic types"
```

---

### Task 3: Replace `Result<_, String>` in lexer

**Files:**
- Modify: `dippin-parser/src/lexer.rs`

**Step 1: Update lexer error sites to emit `Diagnostic` instead of pushing strings**

The lexer today does not return `Result<_, String>` from its public methods (it produces `Token`s and stores diagnostics implicitly via the parser). What we need: a `Vec<Diagnostic>` field on `Lexer`, with helper `push_diagnostic(kind, message, line, col)`. Wire this in place of any existing `eprintln!`/silent path. Specifically:

- Add `pub(crate) diagnostics: Vec<Diagnostic>` to `Lexer`.
- In `read_quoted_content`, when EOL is reached without a closing `"`, push:
  ```rust
  self.diagnostics.push(Diagnostic::error(
      DiagnosticKind::UnterminatedString,
      "unterminated string literal",
      SourceLocation { file: self.filename.clone(), line: self.line, column: start_col },
  ));
  ```
- In `lex_one_token` unknown-char branch, push `DiagnosticKind::UnknownCharacter(...)`.

**Step 2: Build**

```bash
cargo build -p dippin-parser
```

**Step 3: Run tests (existing tests must still pass; new diagnostics not yet surfaced)**

```bash
cargo test -p dippin-parser
```

**Step 4: Commit**

```bash
git add dippin-parser/src/lexer.rs
git commit -m "feat(lexer): collect structured diagnostics for unterminated strings and unknown chars"
```

---

### Task 4: Replace `Result<_, String>` in parser, plumb diagnostics with locations

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Change `Parser::parse` signature**

```rust
pub fn parse(mut self) -> Result<Workflow> {
    // ... existing logic, but instead of self.diagnostics: Vec<String>, use Vec<Diagnostic>
}
```

Replace `self.diagnostics: Vec<String>` with `self.diagnostics: Vec<Diagnostic>`. Drain lexer diagnostics into the parser's diagnostics at the end of parsing.

At return time:
```rust
if !self.diagnostics.is_empty() {
    return Err(Error::Parse {
        file: self.filename.clone(),
        diagnostics: std::mem::take(&mut self.diagnostics),
    });
}
Ok(self.workflow)
```

**Step 2: Replace every `self.diagnostics.push(format!(...))` with structured `Diagnostic`**

For each diagnostic site, identify the appropriate `DiagnosticKind` and pass a `SourceLocation { file, line, column }` from the relevant token. Examples:

- `expect()` failures → `DiagnosticKind::UnexpectedToken { expected, found }`
- `parse_int` failure → `DiagnosticKind::InvalidInteger { value, field }`
- `parse_float` failure → `DiagnosticKind::InvalidFloat { value, field }`
- "unknown defaults field" → `DiagnosticKind::UnknownField { scope: "defaults", name }`
- "unexpected top-level identifier" → `DiagnosticKind::Other`

**Step 3: Build & test**

```bash
cargo build -p dippin-parser
cargo test -p dippin-parser
```

Tests will fail because integration tests still match `Result<_, String>` text. Update those next task.

**Step 4: Commit**

```bash
git add dippin-parser/src/parser.rs
git commit -m "feat(parser): emit structured Diagnostics with source locations"
```

---

### Task 5: Update `lib.rs` public API to use `Result<T>`

**Files:**
- Modify: `dippin-parser/src/lib.rs`

**Step 1: Change function signatures**

```rust
pub fn parse(source: &str, filename: &str) -> Result<Workflow> { ... }
pub fn convert_to_dot(source: &str, filename: &str) -> Result<String> { ... }
pub fn convert_to_dot_with_options(
    source: &str,
    filename: &str,
    opts: &ExportOptions,
) -> Result<String> { ... }
```

**Step 2: Update inline tests in `lib.rs` to use the new error type**

Replace `.expect("should parse")` patterns where appropriate; use `.unwrap()` or pattern-match on `Error::Parse { diagnostics, .. }`.

**Step 3: Update integration tests**

In `dippin-parser/tests/integration_tests.rs`, replace any `Result<_, String>` references with the new types. The `.expect("should parse")` calls remain valid since `Error: std::fmt::Debug`.

**Step 4: Build & test**

```bash
cargo build -p dippin-parser
cargo test -p dippin-parser
```
Expected: all 47 tests still pass.

**Step 5: Commit**

```bash
git add dippin-parser/src/lib.rs dippin-parser/tests/integration_tests.rs
git commit -m "feat(parser): public API now returns Result<T, Error>"
```

---

### Task 6: Update CLI to render diagnostics with `path:line:col`

**Files:**
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Update the `.dip` error path**

Replace:
```rust
let raw_dot = dippin_parser::convert_to_dot(raw_source, &filename).unwrap_or_else(|e| {
    eprintln!("Dippin parse error: {}", e);
    std::process::exit(1);
});
```

With:
```rust
let raw_dot = match dippin_parser::convert_to_dot(raw_source, &filename) {
    Ok(s) => s,
    Err(e) => {
        for diag in e.diagnostics() {
            eprintln!("{}", diag.render());
        }
        if e.diagnostics().is_empty() {
            eprintln!("dippin parse error: {}", e);
        }
        std::process::exit(EX_DATAERR);
    }
};
```

Add at top of `main.rs`:
```rust
const EX_DATAERR: i32 = 65;
const EX_NOINPUT: i32 = 66;
```

**Step 2: Replace `{:?}` Graphviz error with `{}`**

```rust
eprintln!("Graphviz error: {}", e);
```

**Step 3: Build & test CLI**

```bash
cargo build -p dot-viewer-cli
```

**Step 4: Commit**

```bash
git add dot-viewer-cli/src/main.rs
git commit -m "feat(cli): render dippin diagnostics in path:line:col form, distinct exit codes"
```

---

### Task 7: Make `expect()` fail-fast in the parser

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Change `expect` to return `Result`**

```rust
fn expect(&mut self, ttype: TokenType) -> Result<Token> {
    let tok = self.lexer.next_token();
    if tok.token_type != ttype {
        let diag = Diagnostic::error(
            DiagnosticKind::UnexpectedToken {
                expected: format!("{:?}", ttype),
                found: format!("{:?}", tok.token_type),
            },
            format!("expected {:?}, got {:?}", ttype, tok.token_type),
            tok.location.clone(),
        );
        self.diagnostics.push(diag);
        return Err(());
    }
    Ok(tok)
}
```

Where `Err(())` is a sentinel that the production caller uses to skip to the next sync point (newline/outdent). Define a small `type ParseStep<T> = std::result::Result<T, ()>;` for production-level results.

**Step 2: Update each production to propagate the step result**

Each `parse_node`, `parse_workflow`, `parse_parallel`, etc., now returns `ParseStep<()>` and uses `?` on `expect`. On error, the outer loop in `parse_top_level`/`parse_workflow_body` advances to the next newline and continues — this preserves multi-error reporting while preventing the cascading-bogus-diagnostic issue.

**Step 3: Add a sync helper**

```rust
fn sync_to_newline(&mut self) {
    loop {
        let tok = self.lexer.peek_token();
        if matches!(tok.token_type, TokenType::Newline | TokenType::Eof) {
            self.lexer.next_token();
            return;
        }
        self.lexer.next_token();
    }
}
```

**Step 4: Build & test**

```bash
cargo build -p dippin-parser
cargo test -p dippin-parser
```

**Step 5: Commit**

```bash
git add dippin-parser/src/parser.rs
git commit -m "fix(parser): fail-fast on structural errors with sync recovery"
```

---

### Task 8: Validate identifiers after kind keywords

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Add a `expect_identifier(after: &str)` helper**

```rust
fn expect_identifier(&mut self, after: &str) -> ParseStep<Token> {
    let tok = self.lexer.next_token();
    if tok.token_type != TokenType::Identifier {
        self.diagnostics.push(Diagnostic::error(
            DiagnosticKind::MissingIdentifier { after: after.to_string() },
            format!("expected identifier after `{}`, got {:?}", after, tok.token_type),
            tok.location.clone(),
        ));
        return Err(());
    }
    Ok(tok)
}
```

**Step 2: Use it everywhere a bare `next_token().value` is taken as an ID**

- `parse_workflow` (after `workflow` keyword)
- `parse_node` (after kind keyword)
- `parse_parallel` (after `parallel` keyword)
- `parse_fan_in` (after `fan_in` keyword)
- `parse_single_edge` (from/to)

**Step 3: Build & test**

```bash
cargo test -p dippin-parser
```

**Step 4: Commit**

```bash
git add dippin-parser/src/parser.rs
git commit -m "fix(parser): require identifier tokens after kind keywords"
```

---

## Phase 2: Lexer compatibility & UTF-8

### Task 9: Strip UTF-8 BOM in `Lexer::new`

**Files:**
- Modify: `dippin-parser/src/lexer.rs`

**Step 1: Write the failing test**

In the existing `#[cfg(test)] mod tests` block in `lexer.rs`, add:

```rust
#[test]
fn test_lexer_strips_bom() {
    let src = "\u{FEFF}workflow Foo\n";
    let mut lex = Lexer::new(src.to_string(), "test.dip".to_string());
    let tok = lex.next_token();
    assert_eq!(tok.token_type, TokenType::Identifier);
    assert_eq!(tok.value, "workflow");
}
```

**Step 2: Run; expect failure**

```bash
cargo test -p dippin-parser test_lexer_strips_bom
```

**Step 3: Implement**

In `Lexer::new`, before the `split('\n')`:
```rust
let input = input.strip_prefix('\u{FEFF}').map(str::to_string).unwrap_or(input);
```

**Step 4: Run; expect pass**

```bash
cargo test -p dippin-parser test_lexer_strips_bom
```

**Step 5: Commit**

```bash
git add dippin-parser/src/lexer.rs
git commit -m "fix(lexer): strip leading UTF-8 BOM"
```

---

### Task 10: Normalize CRLF and CR-only line endings

**Files:**
- Modify: `dippin-parser/src/lexer.rs`

**Step 1: Failing tests**

```rust
#[test]
fn test_lexer_handles_crlf() {
    let src = "workflow Foo\r\n  goal: bar\r\n";
    let wf = crate::parse(src, "test.dip").expect("CRLF should parse");
    assert_eq!(wf.name, "Foo");
}

#[test]
fn test_lexer_handles_cr_only() {
    let src = "workflow Foo\r  goal: bar\r";
    let wf = crate::parse(src, "test.dip").expect("CR-only should parse");
    assert_eq!(wf.name, "Foo");
}
```

**Step 2: Run; expect failure**

**Step 3: Implement**

In `Lexer::new`, normalize before splitting:
```rust
let input = input.replace("\r\n", "\n").replace('\r', "\n");
```

**Step 4: Run; expect pass**

```bash
cargo test -p dippin-parser test_lexer_handles_crlf test_lexer_handles_cr_only
```

**Step 5: Commit**

```bash
git add dippin-parser/src/lexer.rs
git commit -m "fix(lexer): normalize CRLF and CR-only line endings"
```

---

### Task 11: Enforce indentation rule (no tab/space mixing)

**Files:**
- Modify: `dippin-parser/src/lexer.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_lexer_rejects_mixed_indent() {
    // tab then 2 spaces — clearly mixed
    let src = "workflow Foo\n\t  goal: bar\n";
    let err = crate::parse(src, "test.dip").unwrap_err();
    assert!(err.diagnostics().iter().any(|d| matches!(d.kind, crate::DiagnosticKind::InvalidIndentation(_))));
}
```

**Step 2: Run; expect failure**

**Step 3: Implement**

Add to `lexer.rs`:
```rust
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
```

Call it in `lex_one_line` before computing `indent`.

**Step 4: Run; expect pass**

**Step 5: Commit**

```bash
git add dippin-parser/src/lexer.rs
git commit -m "fix(lexer): diagnose mixed tab/space indentation"
```

---

### Task 12: Diagnose invalid dedents

**Files:**
- Modify: `dippin-parser/src/lexer.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_lexer_rejects_invalid_dedent() {
    // dedent to a level that was never pushed
    let src = "workflow Foo\n    goal: bar\n  exit: x\n";
    let err = crate::parse(src, "test.dip").unwrap_err();
    assert!(err.diagnostics().iter().any(|d| matches!(d.kind, crate::DiagnosticKind::InvalidIndentation(_))));
}
```

**Step 2: Implement**

In `emit_indent_tokens`, after the pop loop, check if `indent != *self.indent_stack.last().unwrap()`:
```rust
if indent != *self.indent_stack.last().unwrap() {
    self.diagnostics.push(Diagnostic::error(
        DiagnosticKind::InvalidIndentation(format!(
            "dedent to column {} does not match any enclosing block",
            indent
        )),
        "invalid dedent",
        SourceLocation { file: self.filename.clone(), line: self.line, column: 1 },
    ));
}
```

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/lexer.rs
git commit -m "fix(lexer): diagnose invalid dedents"
```

---

### Task 13: Char-offset columns instead of byte offsets

**Files:**
- Modify: `dippin-parser/src/lexer.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_lexer_columns_are_char_offsets() {
    // 'é' is 2 bytes in UTF-8
    let src = "workflow Föo\n  exit: \"é\"\n";
    let mut lex = Lexer::new(src.to_string(), "t.dip".to_string());
    // walk to the second line's `exit` identifier
    while lex.peek_token().token_type != TokenType::Eof {
        let t = lex.next_token();
        if t.value == "exit" {
            assert_eq!(t.location.column, 3, "exit should be at column 3 in chars");
            return;
        }
    }
    panic!("did not find exit token");
}
```

**Step 2: Implement**

Replace byte-index column math in `lex_line` with char counts. The simplest correct approach: track `char_col` alongside `i` while iterating. Use `line[..i].chars().count() + col_offset` when emitting a token, OR maintain `char_col` directly.

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/lexer.rs
git commit -m "fix(lexer): report column as character offset, not byte offset"
```

---

### Task 14: Remove remaining `as char` byte casts

**Files:**
- Modify: `dippin-parser/src/lexer.rs`

**Step 1: Audit**

```bash
grep -n 'as char' dippin-parser/src/lexer.rs
```

Expected hits: lines around 405 and 451 (operator/punctuation paths).

**Step 2: Replace with `&'static str` lookup**

Where the byte is known to be one of a small set of ASCII punctuation chars, use a `match ch { b':' => ":", b'=' => "=", ... }` returning `&'static str` to construct the token value, eliminating the cast.

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/lexer.rs
git commit -m "refactor(lexer): remove `as char` byte casts in punctuation paths"
```

---

### Task 15: Fix `find_unquoted_hash` UTF-8-aware advance

**Files:**
- Modify: `dippin-parser/src/lexer.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_find_unquoted_hash_utf8_safe() {
    // backslash followed by multi-byte char inside a string, then a # outside
    let src = r#"label "a\é" # comment"#;
    let result = find_unquoted_hash(src);
    let expected = src.find("# comment").unwrap();
    assert_eq!(result, Some(expected));
}
```

(Make `find_unquoted_hash` `pub(crate)` if not already, so the test can call it.)

**Step 2: Implement**

Iterate via `char_indices` instead of bytes when inside a quoted region; advance by the actual length of the escaped char:

```rust
fn find_unquoted_hash(line: &str) -> Option<usize> {
    let mut chars = line.char_indices().peekable();
    let mut in_quote = false;
    while let Some((i, ch)) = chars.next() {
        if in_quote {
            if ch == '\\' {
                chars.next(); // consume escaped char regardless of width
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
```

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser test_find_unquoted_hash_utf8_safe
git add dippin-parser/src/lexer.rs
git commit -m "fix(lexer): UTF-8-safe escape advance in find_unquoted_hash"
```

---

### Task 16: UTF-8 round-trip regression test

**Files:**
- Modify: `dippin-parser/tests/integration_tests.rs`
- Create: `dippin-parser/testdata/unicode.dip`

**Step 1: Create the fixture**

`dippin-parser/testdata/unicode.dip`:
```
workflow Unicode
  start: Ask
  exit: Done

agent Ask
  prompt: "héllo 你好 🎉"
  model: claude-sonnet-4-6
  provider: anthropic

agent Done
  prompt: "résumé"
  model: gpt-4.1-nano
  provider: openai

edges
  Ask -> Done
```

**Step 2: Add the test**

```rust
#[test]
fn test_parse_unicode() {
    let source = read_testdata("unicode.dip");
    let wf = parse(&source, "unicode.dip").expect("unicode should parse");
    assert_eq!(wf.name, "Unicode");
    let ask = wf.nodes.iter().find(|n| n.id == "Ask").unwrap();
    let dippin_parser::ir::NodeConfig::Agent(cfg) = &ask.config else { panic!() };
    assert_eq!(cfg.prompt, "héllo 你好 🎉");
}

#[test]
fn test_convert_unicode_to_dot() {
    let source = read_testdata("unicode.dip");
    let dot = convert_to_dot_with_options(
        &source,
        "unicode.dip",
        &ExportOptions { include_prompts: true, ..Default::default() },
    ).expect("convert");
    assert!(dot.contains("héllo 你好 🎉") || dot.contains("h\\u00e9llo"));
    assert!(dot.contains("résumé") || dot.contains("r\\u00e9sum"));
}
```

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser test_parse_unicode test_convert_unicode_to_dot
git add dippin-parser/testdata/unicode.dip dippin-parser/tests/integration_tests.rs
git commit -m "test(parser): UTF-8 round-trip regression coverage for a718056"
```

---

## Phase 3: Parser correctness & Go parity

### Task 17: Match Go behavior on unknown edge attributes (silent)

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_unknown_edge_attribute_is_silent() {
    // Go reference silently ignores unknown edge attributes
    let src = "workflow F\n  start: A\n  exit: B\nagent A\n  prompt: x\n  model: m\n  provider: p\nagent B\n  prompt: y\n  model: m\n  provider: p\nedges\n  A -> B foo: bar\n";
    crate::parse(src, "t.dip").expect("unknown edge attr should be ignored");
}
```

**Step 2: Implement**

In the edge-attribute dispatch loop, change the `_ => self.diagnostics.push(...)` arm to consume tokens and `continue` silently. Add a `// ABOUTME` comment line referencing Go parity.

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser test_unknown_edge_attribute_is_silent
git add dippin-parser/src/parser.rs
git commit -m "fix(parser): silently ignore unknown edge attributes (Go parity)"
```

---

### Task 18: Drop `\n`/`\t`/`\r` translation in `unquote_raw` to match Go

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_unquote_raw_only_handles_quote_and_backslash() {
    // Go's unquoteRaw only handles \" and \\
    let result = unquote_raw(r#""line1\nline2""#);
    assert_eq!(result, r"line1\nline2");
}
```

**Step 2: Update `unquote_raw`**

Remove the `\n`/`\t`/`\r` arms; only translate `\"` → `"` and `\\` → `\`.

**Step 3: Update any test that depended on the old translation**

Check `cargo test -p dippin-parser` output and update any failing assertions to match the new (Go-parity) behavior.

**Step 4: Commit**

```bash
git add dippin-parser/src/parser.rs
git commit -m "fix(parser): unquote_raw handles only \\\" and \\\\ (Go parity)"
```

---

### Task 19: Parse durations as structured values

**Files:**
- Create: `dippin-parser/src/duration.rs`
- Modify: `dippin-parser/src/parser.rs`, `dippin-parser/src/ir.rs`, `dippin-parser/src/lib.rs`, `dippin-parser/src/export_dot.rs`

**Step 1: Create `duration.rs`**

```rust
// ABOUTME: Duration newtype with Go-style parsing (e.g., "30s", "5m", "1h30m").
// ABOUTME: Replaces stringly-typed duration fields in IR.

use std::fmt;
use std::time::Duration as StdDuration;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Duration(pub StdDuration);

impl Duration {
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Ok(Duration(StdDuration::ZERO));
        }
        let mut total = StdDuration::ZERO;
        let mut rest = s;
        while !rest.is_empty() {
            let num_end = rest.find(|c: char| !c.is_ascii_digit() && c != '.')
                .ok_or_else(|| format!("invalid duration: {}", s))?;
            if num_end == 0 {
                return Err(format!("invalid duration: {}", s));
            }
            let n: f64 = rest[..num_end].parse().map_err(|_| format!("invalid duration number: {}", &rest[..num_end]))?;
            let unit_end = rest[num_end..]
                .find(|c: char| c.is_ascii_digit())
                .map(|p| num_end + p)
                .unwrap_or(rest.len());
            let unit = &rest[num_end..unit_end];
            let nanos = match unit {
                "ns" => n,
                "us" | "µs" => n * 1_000.0,
                "ms" => n * 1_000_000.0,
                "s" => n * 1_000_000_000.0,
                "m" => n * 60.0 * 1_000_000_000.0,
                "h" => n * 3600.0 * 1_000_000_000.0,
                _ => return Err(format!("unknown duration unit: {}", unit)),
            };
            total += StdDuration::from_nanos(nanos as u64);
            rest = &rest[unit_end..];
        }
        Ok(Duration(total))
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.0.as_secs();
        if secs == 0 && self.0.as_millis() > 0 {
            write!(f, "{}ms", self.0.as_millis())
        } else if secs % 3600 == 0 && secs > 0 {
            write!(f, "{}h", secs / 3600)
        } else if secs % 60 == 0 && secs > 0 {
            write!(f, "{}m", secs / 60)
        } else {
            write!(f, "{}s", secs)
        }
    }
}
```

**Step 2: Wire module into lib.rs**

```rust
pub mod duration;
pub use duration::Duration;
```

**Step 3: Change IR fields**

In `ir.rs`, change:
- `RetryConfig.base_delay: String` → `Duration`
- `AgentConfig.cmd_timeout: String` → `Duration`
- `ToolConfig.timeout: String` → `Duration`

**Step 4: Update parser to call `Duration::parse` and emit `InvalidDuration` diagnostic on failure**

**Step 5: Update `export_dot.rs` to format durations via `Display`**

**Step 6: Update tests that referenced these fields as strings**

**Step 7: Build, test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/duration.rs dippin-parser/src/ir.rs dippin-parser/src/parser.rs dippin-parser/src/lib.rs dippin-parser/src/export_dot.rs dippin-parser/tests/integration_tests.rs
git commit -m "feat(parser): structured Duration type with Go-style parsing"
```

---

### Task 20: Add `Workflow.version` field

**Files:**
- Modify: `dippin-parser/src/ir.rs`, `dippin-parser/src/parser.rs`

**Step 1: Add field**

In `Workflow`:
```rust
pub version: String,
```

**Step 2: Wire in parser**

In `parse_workflow_string_field` (or wherever workflow-level scalar fields dispatch), add:
```rust
"version" => self.workflow.version = val.to_string(),
```

**Step 3: Add a test**

```rust
#[test]
fn test_workflow_version_field() {
    let src = "workflow F\n  version: 1.0\n  start: A\n  exit: A\nagent A\n  prompt: x\n  model: m\n  provider: p\n";
    let wf = crate::parse(src, "t.dip").unwrap();
    assert_eq!(wf.version, "1.0");
}
```

**Step 4: Test, commit**

```bash
cargo test -p dippin-parser test_workflow_version_field
git add dippin-parser/src/ir.rs dippin-parser/src/parser.rs
git commit -m "feat(parser): support Workflow.version field (Go parity)"
```

---

### Task 21: Populate edge/parallel/fan_in source locations

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_edge_has_source_location() {
    let src = "workflow F\n  start: A\n  exit: B\nagent A\n  prompt: x\n  model: m\n  provider: p\nagent B\n  prompt: y\n  model: m\n  provider: p\nedges\n  A -> B\n";
    let wf = crate::parse(src, "t.dip").unwrap();
    let edge = wf.edges.iter().find(|e| e.from == "A" && e.to == "B").unwrap();
    assert_ne!(edge.source.line, 0, "edge should have a real source location");
}
```

**Step 2: Implement**

In `parse_single_edge`, capture `self.lexer.peek_token().location` *before* consuming the `from` identifier and store it on the `Edge`. Same for `parse_parallel`, `parse_fan_in`.

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser test_edge_has_source_location
git add dippin-parser/src/parser.rs
git commit -m "fix(parser): populate source locations on edges, parallel, fan_in"
```

---

### Task 22: Add `ExecutionPath` to `ExportOptions`

**Files:**
- Modify: `dippin-parser/src/export_dot.rs`

**Step 1: Add field**

```rust
#[derive(Debug, Clone, Default)]
pub struct ExportOptions {
    pub include_prompts: bool,
    pub rank_dir: String,
    pub highlight_goal_gates: bool,
    pub execution_path: Vec<String>,
}
```

**Step 2: Implement highlighting**

In `write_node_dot`, if `node.id` appears in `opts.execution_path`, prefix the label with `[N]` (where N is the 1-based index in the path) and append `style="bold,filled", fillcolor="#e0f0ff"` to the attributes.

**Step 3: Add a test**

```rust
#[test]
fn test_export_with_execution_path() {
    let src = "workflow F\n  start: A\n  exit: B\nagent A\n  prompt: x\n  model: m\n  provider: p\nagent B\n  prompt: y\n  model: m\n  provider: p\nedges\n  A -> B\n";
    let opts = ExportOptions { execution_path: vec!["A".into(), "B".into()], ..Default::default() };
    let dot = crate::convert_to_dot_with_options(src, "t.dip", &opts).unwrap();
    assert!(dot.contains("[1]"));
    assert!(dot.contains("[2]"));
    assert!(dot.contains("fillcolor"));
}
```

**Step 4: Test, commit**

```bash
cargo test -p dippin-parser test_export_with_execution_path
git add dippin-parser/src/export_dot.rs dippin-parser/tests/integration_tests.rs
git commit -m "feat(export): ExecutionPath highlighting (Go parity)"
```

---

### Task 23: Match Go DOT keyword quoting (do not quote `node`/`edge`/etc.)

**Files:**
- Modify: `dippin-parser/src/export_dot.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_dot_id_does_not_quote_dot_keywords() {
    // Go reference does not quote `node`, `edge`, etc.
    assert_eq!(dot_id("node"), "node");
    assert_eq!(dot_id("edge"), "edge");
    assert_eq!(dot_id("subgraph"), "subgraph");
}
```

**Step 2: Implement**

Remove the `DOT_RESERVED` list check from `is_simple_dot_id`. Add a comment: `// Go parity: dippin-lang's parser does not quote DOT keyword identifiers.`

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/export_dot.rs
git commit -m "fix(export): do not quote DOT reserved keywords (Go parity)"
```

---

### Task 24: Remove dead `Condition.parsed` field

**Files:**
- Modify: `dippin-parser/src/ir.rs`, `dippin-parser/src/parser.rs`, `dippin-parser/src/export_dot.rs`

**Step 1: Verify it's dead**

```bash
grep -n "parsed" dippin-parser/src/parser.rs
```
Should show only construction sites that set `parsed: None`.

**Step 2: Remove**

- Delete `parsed: Option<ConditionExpr>` from `Condition`.
- Remove the `ConditionExpr` enum from `ir.rs` (and `format_condition_expr` from `export_dot.rs` if dead).
- Update `Condition` to be just `pub struct Condition { pub raw: String }`.
- Update construction sites and the export-side fallback to use `cond.raw` directly.

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/ir.rs dippin-parser/src/parser.rs dippin-parser/src/export_dot.rs
git commit -m "refactor(ir): remove dead Condition.parsed and ConditionExpr"
```

---

### Task 25: Use `u32` for counts

**Files:**
- Modify: `dippin-parser/src/ir.rs`, `dippin-parser/src/parser.rs`

**Step 1: Change types**

In `ir.rs`:
- `WorkflowDefaults.max_retries: i32` → `u32`
- `WorkflowDefaults.max_restarts: i32` → `u32`
- `RetryConfig.max_retries: i32` → `u32`
- `AgentConfig.max_turns: i32` → `u32`
- `Edge.weight: i32` → `u32`

**Step 2: Update parser**

`parse_int` returning `i32` becomes `parse_u32` (or similar). Negative input → `InvalidInteger` diagnostic.

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/ir.rs dippin-parser/src/parser.rs
git commit -m "refactor(ir): use u32 for retry/turn/weight counts"
```

---

### Task 26: Diagnose unknown node/branch fields

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_unknown_agent_field_diagnoses() {
    let src = "workflow F\n  start: A\n  exit: A\nagent A\n  prompt: x\n  bogus: 1\n  model: m\n  provider: p\n";
    let err = crate::parse(src, "t.dip").unwrap_err();
    assert!(err.diagnostics().iter().any(|d| matches!(d.kind, crate::DiagnosticKind::UnknownField { .. })));
}
```

**Step 2: Implement**

Replace `_ => {}` arms in `apply_agent_field`, `apply_human_field`, `apply_tool_field`, `apply_subgraph_field`, `apply_branch_field` with:
```rust
unknown => self.diagnostics.push(Diagnostic::error(
    DiagnosticKind::UnknownField { scope: "agent".into(), name: unknown.into() },
    format!("unknown agent field `{}`", unknown),
    location.clone(),
)),
```

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/parser.rs
git commit -m "fix(parser): diagnose unknown node, branch, and tool fields"
```

---

### Task 27: Diagnose invalid bool values

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_invalid_bool_diagnoses() {
    let src = "workflow F\n  start: A\n  exit: A\nagent A\n  prompt: x\n  goal_gate: yes\n  model: m\n  provider: p\n";
    let err = crate::parse(src, "t.dip").unwrap_err();
    assert!(err.diagnostics().iter().any(|d| matches!(d.kind, crate::DiagnosticKind::InvalidBool { .. })));
}
```

**Step 2: Add `parse_bool` helper**

```rust
fn parse_bool(&mut self, value: &str, field: &str, loc: &SourceLocation) -> bool {
    match value {
        "true" => true,
        "false" => false,
        _ => {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::InvalidBool { value: value.into(), field: field.into() },
                format!("`{}` requires true or false, got `{}`", field, value),
                loc.clone(),
            ));
            false
        }
    }
}
```

Replace every `val == "true"` with `self.parse_bool(val, "field_name", &loc)`.

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/parser.rs
git commit -m "fix(parser): diagnose invalid boolean field values"
```

---

### Task 28: Diagnose multiple top-level workflows

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_duplicate_workflow_diagnoses() {
    let src = "workflow F\n  start: A\n  exit: A\nagent A\n  prompt: x\n  model: m\n  provider: p\nworkflow G\n";
    let err = crate::parse(src, "t.dip").unwrap_err();
    assert!(err.diagnostics().iter().any(|d| matches!(d.kind, crate::DiagnosticKind::DuplicateWorkflow)));
}
```

**Step 2: Implement**

In `parse_workflow`, if `self.workflow.name` is non-empty, push `DiagnosticKind::DuplicateWorkflow`.

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/parser.rs
git commit -m "fix(parser): diagnose multiple top-level workflow declarations"
```

---

### Task 29: Diagnose empty workflow

**Files:**
- Modify: `dippin-parser/src/parser.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_empty_file_diagnoses() {
    let err = crate::parse("", "empty.dip").unwrap_err();
    assert!(err.diagnostics().iter().any(|d| matches!(d.kind, crate::DiagnosticKind::EmptyWorkflow)));
}
```

**Step 2: Implement**

After parsing, if `self.workflow.name.is_empty() && self.workflow.nodes.is_empty()`:
```rust
self.diagnostics.push(Diagnostic::error(
    DiagnosticKind::EmptyWorkflow,
    "no workflow declared",
    SourceLocation { file: self.filename.clone(), line: 1, column: 1 },
));
```

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/parser.rs
git commit -m "fix(parser): diagnose empty workflow input"
```

---

### Task 30: Semantic validation pass — start/exit/edges reference declared nodes

**Files:**
- Create: `dippin-parser/src/validate.rs`
- Modify: `dippin-parser/src/lib.rs`, `dippin-parser/src/parser.rs`

**Step 1: Create the module**

```rust
// ABOUTME: Semantic validation pass run after parsing.
// ABOUTME: Verifies start/exit/edge references point at declared nodes.

use crate::error::{Diagnostic, DiagnosticKind};
use crate::ir::Workflow;
use std::collections::HashSet;

pub fn validate(wf: &Workflow, file: &str) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let ids: HashSet<&str> = wf.nodes.iter().map(|n| n.id.as_str()).collect();

    let mut check = |name: &str, target: &str, line: usize| {
        if !target.is_empty() && !ids.contains(target) {
            diags.push(Diagnostic::error(
                DiagnosticKind::UndefinedNodeReference(target.into()),
                format!("{} references undefined node `{}`", name, target),
                crate::ir::SourceLocation { file: file.into(), line, column: 1 },
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
```

**Step 2: Wire into parser pipeline**

In `Parser::parse`, after building `self.workflow` and before the diagnostics-empty check:
```rust
self.diagnostics.extend(crate::validate::validate(&self.workflow, &self.filename));
```

**Step 3: Failing test**

```rust
#[test]
fn test_undefined_node_reference_diagnoses() {
    let src = "workflow F\n  start: Missing\n  exit: A\nagent A\n  prompt: x\n  model: m\n  provider: p\n";
    let err = crate::parse(src, "t.dip").unwrap_err();
    assert!(err.diagnostics().iter().any(|d| matches!(d.kind, crate::DiagnosticKind::UndefinedNodeReference(_))));
}
```

**Step 4: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/validate.rs dippin-parser/src/lib.rs dippin-parser/src/parser.rs
git commit -m "feat(parser): semantic validation of node references"
```

---

## Phase 4: Security & DOT export

### Task 31: Fix `dot_quote` to escape all backslashes

**Files:**
- Modify: `dippin-parser/src/export_dot.rs`

**Step 1: Failing test**

```rust
#[test]
fn test_dot_quote_escapes_all_backslashes() {
    // Inputs with literal backslashes must produce \\, not be passed through
    assert_eq!(dot_quote(r"path\to\file"), r#""path\\to\\file""#);
    // Embedded quote must be escaped
    assert_eq!(dot_quote(r#"a"b"#), r#""a\"b""#);
    // Real newline must be escaped
    assert_eq!(dot_quote("line1\nline2"), r#""line1\nline2""#);
}
```

**Step 2: Implement**

Rewrite `dot_quote` as a strict escaper:
```rust
pub fn dot_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
```

Remove the `is_dot_escape_char` helper and the special-case lookahead logic.

**Step 3: Update any tests that depended on the old "preserve `\n`/`\l`/`\r`" behavior**

**Step 4: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/export_dot.rs
git commit -m "fix(export): dot_quote escapes all backslashes and control chars"
```

---

### Task 32: Apply escaping consistently to all values

**Files:**
- Modify: `dippin-parser/src/export_dot.rs`

**Step 1: Audit**

Every place that emits a DOT attribute value should pass the value through `dot_quote`. Search for any direct `format!("{}={}", k, v)` patterns where `v` bypasses `dot_quote`.

**Step 2: Remove the `escape_newlines` helper**

It's now redundant — `dot_quote` handles `\n` and `\r`. Delete the function and any callers, replacing with `dot_quote`.

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/export_dot.rs
git commit -m "refactor(export): consolidate DOT escaping in dot_quote"
```

---

### Task 33: Add input size limit

**Files:**
- Modify: `dippin-parser/src/lib.rs`, `dippin-parser/src/error.rs`

**Step 1: Add a constant and check**

In `lib.rs`:
```rust
/// Maximum source file size in bytes accepted by `parse`.
pub const MAX_INPUT_SIZE: usize = 10 * 1024 * 1024;
```

In `parse`:
```rust
if source.len() > MAX_INPUT_SIZE {
    return Err(Error::Parse {
        file: filename.into(),
        diagnostics: vec![Diagnostic::error(
            DiagnosticKind::Other,
            format!("input exceeds maximum size of {} bytes", MAX_INPUT_SIZE),
            crate::ir::SourceLocation { file: filename.into(), line: 1, column: 1 },
        )],
    });
}
```

**Step 2: Add a test**

```rust
#[test]
fn test_oversize_input_rejected() {
    let big = "a".repeat(crate::MAX_INPUT_SIZE + 1);
    assert!(crate::parse(&big, "big.dip").is_err());
}
```

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser test_oversize_input_rejected
git add dippin-parser/src/lib.rs
git commit -m "feat(parser): cap input size at 10 MiB"
```

---

### Task 34: Cap indent depth

**Files:**
- Modify: `dippin-parser/src/lexer.rs`

**Step 1: Add constant**

```rust
const MAX_INDENT_DEPTH: usize = 64;
```

**Step 2: Enforce in `emit_indent_tokens`**

When pushing a new level, if `self.indent_stack.len() >= MAX_INDENT_DEPTH`, push a diagnostic and refuse the push.

**Step 3: Test**

```rust
#[test]
fn test_indent_depth_capped() {
    let mut src = String::new();
    for i in 0..100 {
        src.push_str(&" ".repeat(i));
        src.push_str("a:\n");
    }
    let _err = crate::parse(&src, "deep.dip").unwrap_err();
}
```

**Step 4: Commit**

```bash
git add dippin-parser/src/lexer.rs
git commit -m "feat(lexer): cap indentation depth at 64 levels"
```

---

### Task 35: Refactor `read_condition_raw` to use lexer raw text

**Files:**
- Modify: `dippin-parser/src/parser.rs`, `dippin-parser/src/lexer.rs`

**Step 1: Use `Lexer::raw_value_text(line)` for the condition**

Today `read_condition_raw` joins individual tokens with spaces, losing original spacing. Instead, capture the original substring of the source line from the column after `when` to end-of-line.

**Step 2: Add a test confirming spacing is preserved**

```rust
#[test]
fn test_condition_preserves_original_spacing() {
    let src = "workflow F\n  start: A\n  exit: B\nagent A\n  prompt: x\n  model: m\n  provider: p\nagent B\n  prompt: y\n  model: m\n  provider: p\nedges\n  A -> B when ctx.x==success\n";
    let wf = crate::parse(src, "t.dip").unwrap();
    let edge = &wf.edges[0];
    assert_eq!(edge.condition.as_ref().unwrap().raw, "ctx.x==success");
}
```

**Step 3: Test, commit**

```bash
cargo test -p dippin-parser
git add dippin-parser/src/parser.rs dippin-parser/src/lexer.rs
git commit -m "fix(parser): preserve original spacing in edge conditions"
```

---

### Task 36: CLI input size guard

**Files:**
- Modify: `dot-viewer-cli/src/main.rs`

**Step 1: Check file size before reading**

```rust
let metadata = std::fs::metadata(&cli.file).unwrap_or_else(|e| {
    eprintln!("error: cannot stat {}: {}", cli.file.display(), e);
    std::process::exit(EX_NOINPUT);
});
if metadata.len() as usize > dippin_parser::MAX_INPUT_SIZE {
    eprintln!("error: file exceeds maximum size of {} bytes", dippin_parser::MAX_INPUT_SIZE);
    std::process::exit(EX_DATAERR);
}
```

**Step 2: Build, commit**

```bash
cargo build -p dot-viewer-cli
git add dot-viewer-cli/src/main.rs
git commit -m "feat(cli): reject oversize input files"
```

---

## Final verification

### Task 37: Full test sweep + clippy

**Files:** none

**Step 1: Run all tests**

```bash
cargo test -p dippin-parser
cargo test -p dot-viewer-cli 2>&1 | tail -20
```

**Step 2: Clippy clean**

```bash
cargo clippy -p dippin-parser -- -D warnings
```

**Step 3: If anything fails, fix and recommit. No new commit needed if clean.**

---

## Notes for the executing engineer

- This branch is `dippin-support/omakase/variant-rust-port`. Do **not** rebase.
- The `dot-viewer-cli` crate cannot fully build without `graphviz-vendor` cloned (see project memory). For tasks that don't touch CLI integration, scope tests to `cargo test -p dippin-parser`.
- Go reference at `/Users/dylanr/work/2389/dippin-lang/parser/` is authoritative for parity questions. When in doubt, read the Go source.
- The `superpowers:test-driven-development` skill applies. Each task follows red → green → commit.
- Companion plans run AFTER this one. Some tasks here (`#[non_exhaustive]`, doc comments, `IndexMap`) are deliberately deferred to plan #2.
