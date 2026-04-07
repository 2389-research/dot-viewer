// ABOUTME: Recursive descent parser for the Dippin workflow language.
// ABOUTME: Consumes tokens from the lexer and produces an IR Workflow structure.

use std::collections::HashMap;

use crate::duration::Duration;
use crate::error::{Diagnostic, DiagnosticKind, Error, Result};
use crate::ir::*;
use crate::lexer::{Lexer, Token, TokenType};

/// Per-production result: `Err(())` signals the caller should sync to a recovery point.
type ParseStep<T> = std::result::Result<T, ()>;

/// Parser state holding the lexer, diagnostics, and the workflow being built.
pub struct Parser {
    lexer: Lexer,
    diagnostics: Vec<Diagnostic>,
    workflow: Workflow,
    filename: String,
}

impl Parser {
    /// Create a new parser for the given input.
    pub fn new(input: &str, filename: &str) -> Self {
        Parser {
            lexer: Lexer::new(input, filename),
            diagnostics: Vec::new(),
            workflow: Workflow::default(),
            filename: filename.to_string(),
        }
    }

    /// Parse the input and return a Workflow or an error with diagnostics.
    pub fn parse(mut self) -> Result<Workflow> {
        self.parse_top_level();
        // Drain any diagnostics emitted by the lexer
        let lex_diags = std::mem::take(&mut self.lexer.diagnostics);
        self.diagnostics.extend(lex_diags);
        if !self.diagnostics.is_empty() {
            return Err(Error::Parse {
                file: self.filename.clone(),
                diagnostics: std::mem::take(&mut self.diagnostics),
            });
        }
        Ok(self.workflow)
    }

    /// Consume top-level tokens looking for workflow declarations.
    fn parse_top_level(&mut self) {
        loop {
            let t = self.lexer.peek_token();
            if t.token_type == TokenType::Eof {
                break;
            }
            if t.token_type == TokenType::Newline {
                self.lexer.next_token();
                continue;
            }
            if t.token_type == TokenType::Identifier && t.value == "workflow" {
                if self.parse_workflow().is_err() {
                    self.sync_to_newline();
                }
            } else {
                self.lexer.next_token();
            }
        }
    }

    /// Parse a workflow declaration: workflow Name\n INDENT body OUTDENT
    fn parse_workflow(&mut self) -> ParseStep<()> {
        self.lexer.next_token(); // "workflow"
        let name = self.expect_identifier("workflow")?.value;
        self.workflow.name = name;
        self.expect(TokenType::Newline)?;
        self.expect(TokenType::Indent)?;
        self.parse_workflow_body();
        self.expect(TokenType::Outdent)?;
        Ok(())
    }

    /// Parse the indented body of a workflow declaration.
    fn parse_workflow_body(&mut self) {
        loop {
            let t = self.lexer.peek_token();
            if t.token_type == TokenType::Outdent || t.token_type == TokenType::Eof {
                break;
            }
            if t.token_type == TokenType::Newline {
                self.lexer.next_token();
                continue;
            }
            if t.token_type == TokenType::Identifier {
                if self.dispatch_workflow_field(&t.clone()).is_err() {
                    self.sync_to_newline();
                }
            } else {
                self.lexer.next_token();
            }
        }
    }

    /// Route a workflow-level identifier to the right handler.
    fn dispatch_workflow_field(&mut self, t: &Token) -> ParseStep<()> {
        match t.value.as_str() {
            "goal" | "start" | "exit" | "version" => self.parse_workflow_string_field(t),
            "defaults" => self.parse_defaults(),
            "edges" => self.parse_edges(),
            "stylesheet" => self.parse_stylesheet(),
            "parallel" => self.parse_parallel(),
            "fan_in" => self.parse_fan_in(),
            _ => self.dispatch_workflow_default(t),
        }
    }

    /// Handle node kinds and unknown identifiers.
    fn dispatch_workflow_default(&mut self, t: &Token) -> ParseStep<()> {
        if let Ok(kind) = t.value.parse::<NodeKind>() {
            self.parse_node(kind)
        } else {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::Other,
                format!("unexpected top-level identifier: {}", t.value),
                t.location.clone(),
            ));
            self.lexer.next_token();
            Ok(())
        }
    }

    /// Parse a simple workflow field like "goal: value".
    fn parse_workflow_string_field(&mut self, t: &Token) -> ParseStep<()> {
        let field_name = t.value.clone();
        let line = t.location.line;
        self.lexer.next_token(); // consume field name
        self.expect(TokenType::Colon)?;
        let val = self.read_field_value(line);
        match field_name.as_str() {
            "goal" => self.workflow.goal = val,
            "start" => self.workflow.start = val,
            "exit" => self.workflow.exit = val,
            "version" => self.workflow.version = val,
            _ => {}
        }
        Ok(())
    }

    /// Expect a specific token type, recording a diagnostic if it doesn't match.
    /// Returns `Err(())` on mismatch so callers can sync to a recovery point.
    fn expect(&mut self, expected: TokenType) -> ParseStep<Token> {
        let tok = self.lexer.next_token();
        if tok.token_type != expected {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::UnexpectedToken {
                    expected: format!("{:?}", expected),
                    found: format!("{:?}", tok.token_type),
                },
                format!("expected {:?}, got {:?}", expected, tok.token_type),
                tok.location.clone(),
            ));
            return Err(());
        }
        Ok(tok)
    }

    /// Expect an identifier token after a keyword like `workflow`, `agent`, or an edge side.
    fn expect_identifier(&mut self, after: &str) -> ParseStep<Token> {
        let tok = self.lexer.next_token();
        if tok.token_type != TokenType::Identifier {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::MissingIdentifier {
                    after: after.to_string(),
                },
                format!(
                    "expected identifier after `{}`, got {:?}",
                    after, tok.token_type
                ),
                tok.location.clone(),
            ));
            return Err(());
        }
        Ok(tok)
    }

    /// Advance the token stream past the next newline (or EOF).
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

    /// Read a field value which may be a raw block, newline-then-block, or single-line.
    fn read_field_value(&mut self, line_num: usize) -> String {
        if self.lexer.peek_token().token_type == TokenType::RawBlock {
            return self.lexer.next_token().value;
        }
        if self.lexer.peek_token().token_type == TokenType::Newline {
            self.lexer.next_token(); // consume newline
            if self.lexer.peek_token().token_type == TokenType::RawBlock {
                return self.lexer.next_token().value;
            }
            return String::new();
        }
        self.read_single_line_value(line_num)
    }

    /// Read a single-line value using raw extraction from the lexer.
    fn read_single_line_value(&mut self, line_num: usize) -> String {
        let raw = self.lexer.raw_value_text(line_num);
        self.consume_until_newline();
        unquote_raw(&raw)
    }

    /// Consume tokens until a newline or EOF.
    fn consume_until_newline(&mut self) {
        loop {
            let t = self.lexer.peek_token();
            if t.token_type == TokenType::Newline || t.token_type == TokenType::Eof {
                break;
            }
            self.lexer.next_token();
        }
    }

    /// Parse a comma-separated list of identifiers.
    fn parse_comma_list(&mut self) -> Vec<String> {
        let mut list = vec![self.lexer.next_token().value];
        while self.lexer.peek_token().token_type == TokenType::Comma {
            self.lexer.next_token(); // comma
            list.push(self.lexer.next_token().value);
        }
        list
    }

    // ── Defaults ──────────────────────────────────────────

    /// Parse the defaults block.
    fn parse_defaults(&mut self) -> ParseStep<()> {
        self.lexer.next_token(); // "defaults"
        self.expect(TokenType::Newline)?;
        self.expect(TokenType::Indent)?;
        self.parse_defaults_body();
        self.expect(TokenType::Outdent)?;
        Ok(())
    }

    /// Parse fields within the defaults block.
    fn parse_defaults_body(&mut self) {
        loop {
            let t = self.lexer.peek_token();
            if t.token_type == TokenType::Outdent || t.token_type == TokenType::Eof {
                break;
            }
            if t.token_type == TokenType::Newline {
                self.lexer.next_token();
                continue;
            }
            if t.token_type == TokenType::Identifier {
                if self.parse_defaults_field().is_err() {
                    self.sync_to_newline();
                }
            } else {
                self.lexer.next_token();
            }
        }
    }

    /// Parse a single field within the defaults block.
    fn parse_defaults_field(&mut self) -> ParseStep<()> {
        let t = self.lexer.peek_token();
        let key = t.value.clone();
        let loc = t.location.clone();
        self.lexer.next_token();
        self.expect(TokenType::Colon)?;
        let val = self.read_field_value(loc.line);
        self.apply_default_field(&key, &val, &loc);
        Ok(())
    }

    /// Apply a single default field value.
    fn apply_default_field(&mut self, key: &str, val: &str, loc: &SourceLocation) {
        match key {
            "model" => self.workflow.defaults.model = val.to_string(),
            "provider" => self.workflow.defaults.provider = val.to_string(),
            "retry_policy" => self.workflow.defaults.retry_policy = val.to_string(),
            "fidelity" => self.workflow.defaults.fidelity = val.to_string(),
            "restart_target" => self.workflow.defaults.restart_target = val.to_string(),
            "compaction" => self.workflow.defaults.compaction = val.to_string(),
            "on_resume" => self.workflow.defaults.on_resume = val.to_string(),
            "max_retries" => self.workflow.defaults.max_retries = self.parse_int(val, key, loc),
            "max_restarts" => self.workflow.defaults.max_restarts = self.parse_int(val, key, loc),
            "cache_tools" => self.workflow.defaults.cache_tools = val == "true",
            _ => {
                self.diagnostics.push(Diagnostic::error(
                    DiagnosticKind::UnknownField {
                        scope: "defaults".to_string(),
                        name: key.to_string(),
                    },
                    format!("unknown defaults field {:?}", key),
                    loc.clone(),
                ));
            }
        }
    }

    // ── Nodes ─────────────────────────────────────────────

    /// Parse a node declaration: kind ID\n INDENT fields OUTDENT
    fn parse_node(&mut self, kind: NodeKind) -> ParseStep<()> {
        let kind_tok = self.lexer.next_token(); // kind keyword
        let id = self.expect_identifier(&kind_tok.value)?.value;
        let source = self.lexer.peek_token().location.clone();
        let config = default_node_config(&kind);
        let mut node = Node {
            id,
            kind,
            label: String::new(),
            classes: Vec::new(),
            config,
            retry: RetryConfig::default(),
            io: NodeIO::default(),
            source,
        };
        self.expect(TokenType::Newline)?;
        self.expect(TokenType::Indent)?;
        self.parse_node_body(&mut node);
        self.expect(TokenType::Outdent)?;
        self.workflow.nodes.push(node);
        Ok(())
    }

    /// Parse fields within a node body.
    fn parse_node_body(&mut self, node: &mut Node) {
        loop {
            let t = self.lexer.peek_token();
            if t.token_type == TokenType::Outdent || t.token_type == TokenType::Eof {
                break;
            }
            if t.token_type == TokenType::Newline {
                self.lexer.next_token();
                continue;
            }
            if t.token_type == TokenType::Identifier {
                if self.parse_node_field(node).is_err() {
                    self.sync_to_newline();
                }
            } else {
                self.lexer.next_token();
            }
        }
    }

    /// Parse a single field inside a node body.
    fn parse_node_field(&mut self, node: &mut Node) -> ParseStep<()> {
        let t = self.lexer.peek_token();
        let key = t.value.clone();
        let loc = t.location.clone();
        self.lexer.next_token();
        self.expect(TokenType::Colon)?;
        let val = self.read_field_value(loc.line);
        self.apply_node_field(node, &key, &val, &loc);
        Ok(())
    }

    /// Apply a field to a node, trying common fields first.
    fn apply_node_field(&mut self, node: &mut Node, key: &str, val: &str, loc: &SourceLocation) {
        if self.try_apply_common_field(node, key, val, loc) {
            return;
        }
        self.apply_config_field(node, key, val, loc);
    }

    /// Try to apply common fields (label, class, reads, writes, retry).
    fn try_apply_common_field(
        &mut self,
        node: &mut Node,
        key: &str,
        val: &str,
        loc: &SourceLocation,
    ) -> bool {
        match key {
            "label" => node.label = val.to_string(),
            "class" => node.classes = split_comma(val),
            "reads" => node.io.reads = split_comma(val),
            "writes" => node.io.writes = split_comma(val),
            "retry_policy" => node.retry.policy = val.to_string(),
            "retry_target" => node.retry.retry_target = val.to_string(),
            "fallback_target" => node.retry.fallback_target = val.to_string(),
            "max_retries" => node.retry.max_retries = self.parse_int(val, key, loc),
            "base_delay" => node.retry.base_delay = self.parse_duration(val, key, loc),
            _ => return false,
        }
        true
    }

    /// Dispatch to config-specific field handlers.
    fn apply_config_field(&mut self, node: &mut Node, key: &str, val: &str, loc: &SourceLocation) {
        match &mut node.config {
            NodeConfig::Agent(cfg) => self.apply_agent_field(cfg, key, val, loc),
            NodeConfig::Human(cfg) => apply_human_field(cfg, key, val),
            NodeConfig::Tool(cfg) => self.apply_tool_field(cfg, key, val, loc),
            NodeConfig::Subgraph(cfg) => apply_subgraph_field(cfg, key, val),
            _ => {}
        }
    }

    /// Apply agent-specific configuration fields.
    fn apply_agent_field(
        &mut self,
        cfg: &mut AgentConfig,
        key: &str,
        val: &str,
        loc: &SourceLocation,
    ) {
        match key {
            "prompt" => cfg.prompt = val.to_string(),
            "system_prompt" => cfg.system_prompt = val.to_string(),
            "model" => cfg.model = val.to_string(),
            "provider" => cfg.provider = val.to_string(),
            "reasoning_effort" => cfg.reasoning_effort = val.to_string(),
            "fidelity" => cfg.fidelity = val.to_string(),
            "response_format" => cfg.response_format = val.to_string(),
            "response_schema" => cfg.response_schema = val.to_string(),
            "compaction" => cfg.compaction = val.to_string(),
            "goal_gate" => cfg.goal_gate = val == "true",
            "auto_status" => cfg.auto_status = val == "true",
            "cache_tools" => cfg.cache_tools = val == "true",
            "max_turns" => cfg.max_turns = self.parse_int(val, key, loc),
            "compaction_threshold" => cfg.compaction_threshold = self.parse_float(val, key, loc),
            "cmd_timeout" => cfg.cmd_timeout = self.parse_duration(val, key, loc),
            "params" => cfg.params = parse_params_block(val),
            _ => {}
        }
    }

    /// Apply tool-specific configuration fields.
    fn apply_tool_field(
        &mut self,
        cfg: &mut ToolConfig,
        key: &str,
        val: &str,
        loc: &SourceLocation,
    ) {
        match key {
            "command" => cfg.command = val.to_string(),
            "timeout" => cfg.timeout = self.parse_duration(val, key, loc),
            "outputs" => cfg.outputs = split_comma(val),
            _ => {}
        }
    }

    // ── Parallel / FanIn ──────────────────────────────────

    /// Parse a parallel node (inline or block form).
    fn parse_parallel(&mut self) -> ParseStep<()> {
        self.lexer.next_token(); // "parallel"
        let id = self.expect_identifier("parallel")?.value;

        if self.lexer.peek_token().token_type == TokenType::Arrow {
            return self.parse_parallel_inline(&id);
        }
        self.parse_parallel_block(&id)
    }

    /// Parse inline form: parallel ID -> target, target
    fn parse_parallel_inline(&mut self, id: &str) -> ParseStep<()> {
        self.expect(TokenType::Arrow)?;
        let targets = self.parse_comma_list();
        self.workflow.nodes.push(Node {
            id: id.to_string(),
            kind: NodeKind::Parallel,
            label: String::new(),
            classes: Vec::new(),
            config: NodeConfig::Parallel(ParallelConfig {
                targets,
                branches: Vec::new(),
            }),
            retry: RetryConfig::default(),
            io: NodeIO::default(),
            source: SourceLocation::default(),
        });
        self.expect(TokenType::Newline)?;
        Ok(())
    }

    /// Parse block form with per-branch config.
    fn parse_parallel_block(&mut self, id: &str) -> ParseStep<()> {
        self.expect(TokenType::Newline)?;
        self.expect(TokenType::Indent)?;
        let branches = self.parse_parallel_branches();
        self.expect(TokenType::Outdent)?;

        let targets: Vec<String> = branches.iter().map(|b| b.target.clone()).collect();
        self.workflow.nodes.push(Node {
            id: id.to_string(),
            kind: NodeKind::Parallel,
            label: String::new(),
            classes: Vec::new(),
            config: NodeConfig::Parallel(ParallelConfig { targets, branches }),
            retry: RetryConfig::default(),
            io: NodeIO::default(),
            source: SourceLocation::default(),
        });
        Ok(())
    }

    /// Parse branch declarations inside a parallel block.
    fn parse_parallel_branches(&mut self) -> Vec<BranchConfig> {
        let mut branches = Vec::new();
        loop {
            let t = self.lexer.peek_token();
            if t.token_type == TokenType::Outdent || t.token_type == TokenType::Eof {
                break;
            }
            if t.token_type == TokenType::Newline {
                self.lexer.next_token();
                continue;
            }
            if t.token_type == TokenType::Identifier && t.value == "branch" {
                match self.parse_one_branch() {
                    Ok(bc) => branches.push(bc),
                    Err(()) => self.sync_to_newline(),
                }
            } else {
                self.lexer.next_token();
            }
        }
        branches
    }

    /// Parse: branch: target\n [INDENT fields OUTDENT]
    fn parse_one_branch(&mut self) -> ParseStep<BranchConfig> {
        self.lexer.next_token(); // "branch"
        self.expect(TokenType::Colon)?;
        let target = self.lexer.next_token().value;
        let mut bc = BranchConfig {
            target,
            ..Default::default()
        };
        self.consume_until_newline();

        if self.lexer.peek_token().token_type == TokenType::Newline {
            self.lexer.next_token();
        }
        if self.lexer.peek_token().token_type != TokenType::Indent {
            return Ok(bc);
        }
        self.expect(TokenType::Indent)?;
        self.parse_branch_fields(&mut bc);
        self.expect(TokenType::Outdent)?;
        Ok(bc)
    }

    /// Parse fields within a branch block.
    fn parse_branch_fields(&mut self, bc: &mut BranchConfig) {
        loop {
            let t = self.lexer.peek_token();
            if t.token_type == TokenType::Outdent || t.token_type == TokenType::Eof {
                break;
            }
            if t.token_type == TokenType::Newline {
                self.lexer.next_token();
                continue;
            }
            if t.token_type == TokenType::Identifier {
                if self.parse_branch_field(bc).is_err() {
                    self.sync_to_newline();
                }
            } else {
                self.lexer.next_token();
            }
        }
    }

    /// Parse a single field within a branch block.
    fn parse_branch_field(&mut self, bc: &mut BranchConfig) -> ParseStep<()> {
        let t = self.lexer.peek_token();
        let key = t.value.clone();
        let line = t.location.line;
        self.lexer.next_token();
        self.expect(TokenType::Colon)?;
        let val = self.read_field_value(line);
        match key.as_str() {
            "model" => bc.model = val,
            "provider" => bc.provider = val,
            "fidelity" => bc.fidelity = val,
            _ => {}
        }
        Ok(())
    }

    /// Parse a fan_in node: fan_in ID <- source, source
    fn parse_fan_in(&mut self) -> ParseStep<()> {
        self.lexer.next_token(); // "fan_in"
        let id = self.expect_identifier("fan_in")?.value;
        self.expect(TokenType::BackArrow)?;
        let sources = self.parse_comma_list();
        self.workflow.nodes.push(Node {
            id: id.to_string(),
            kind: NodeKind::FanIn,
            label: String::new(),
            classes: Vec::new(),
            config: NodeConfig::FanIn(FanInConfig { sources }),
            retry: RetryConfig::default(),
            io: NodeIO::default(),
            source: SourceLocation::default(),
        });
        self.expect(TokenType::Newline)?;
        Ok(())
    }

    // ── Edges ─────────────────────────────────────────────

    /// Parse the edges section.
    fn parse_edges(&mut self) -> ParseStep<()> {
        self.lexer.next_token(); // "edges"
        self.expect(TokenType::Newline)?;
        self.expect(TokenType::Indent)?;
        self.parse_edges_body();
        self.expect(TokenType::Outdent)?;
        Ok(())
    }

    /// Parse the body of an edges block.
    fn parse_edges_body(&mut self) {
        loop {
            let t = self.lexer.peek_token();
            if t.token_type == TokenType::Outdent || t.token_type == TokenType::Eof {
                break;
            }
            if t.token_type == TokenType::Newline {
                self.lexer.next_token();
                continue;
            }
            if self.parse_single_edge().is_err() {
                self.sync_to_newline();
            }
        }
    }

    /// Parse a single edge: from -> to [attributes...]
    fn parse_single_edge(&mut self) -> ParseStep<()> {
        let from = self.expect_identifier("edge from")?.value;
        self.expect(TokenType::Arrow)?;
        let to = self.expect_identifier("->")?.value;
        let mut edge = Edge {
            from,
            to,
            label: String::new(),
            condition: None,
            weight: 0,
            restart: false,
            source: SourceLocation::default(),
        };
        self.parse_edge_attributes(&mut edge)?;
        self.workflow.edges.push(edge);
        self.expect(TokenType::Newline)?;
        Ok(())
    }

    /// Parse optional edge attributes (when, label, weight, restart).
    fn parse_edge_attributes(&mut self, edge: &mut Edge) -> ParseStep<()> {
        loop {
            let t = self.lexer.peek_token();
            if t.token_type == TokenType::Newline || t.token_type == TokenType::Eof {
                break;
            }
            let attr = self.lexer.next_token();
            match attr.value.as_str() {
                "when" => {
                    let raw = self.read_condition_raw();
                    edge.condition = Some(Condition { raw, parsed: None });
                }
                "label" => {
                    self.expect(TokenType::Colon)?;
                    edge.label = self.lexer.next_token().value;
                }
                "weight" => {
                    self.expect(TokenType::Colon)?;
                    let wt = self.lexer.next_token();
                    let wt_loc = wt.location.clone();
                    edge.weight = self.parse_int(&wt.value, "weight", &wt_loc);
                }
                "restart" => {
                    self.expect(TokenType::Colon)?;
                    edge.restart = self.lexer.next_token().value == "true";
                }
                _ => {
                    // Go parity: dippin-lang silently ignores unknown edge
                    // attributes; consume any value tokens up to the next
                    // attribute keyword or end-of-line and continue.
                    let _ = attr;
                    if self.lexer.peek_token().token_type == TokenType::Colon {
                        self.lexer.next_token();
                        if !matches!(
                            self.lexer.peek_token().token_type,
                            TokenType::Newline | TokenType::Eof
                        ) {
                            self.lexer.next_token();
                        }
                    }
                    continue;
                }
            }
        }
        Ok(())
    }

    /// Read tokens for a condition expression until newline/EOF or a known edge keyword.
    fn read_condition_raw(&mut self) -> String {
        let edge_attr_keywords = ["label", "weight", "restart"];
        let mut parts = Vec::new();
        loop {
            let pk = self.lexer.peek_token();
            if pk.token_type == TokenType::Newline || pk.token_type == TokenType::Eof {
                break;
            }
            if edge_attr_keywords.contains(&pk.value.as_str()) {
                break;
            }
            let t = self.lexer.next_token();
            if t.token_type == TokenType::Literal {
                parts.push(format!("\"{}\"", t.value));
            } else {
                parts.push(t.value);
            }
        }
        parts.join(" ").trim().to_string()
    }

    // ── Stylesheet ────────────────────────────────────────

    /// Parse the stylesheet section.
    fn parse_stylesheet(&mut self) -> ParseStep<()> {
        self.lexer.next_token(); // "stylesheet"
        self.expect(TokenType::Colon)?;
        let line = self.lexer.peek_token().location.line;
        let val = self.read_field_value(line);
        self.workflow.stylesheet = parse_stylesheet_raw(&val);
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────

    /// Parse an integer from a string, recording a diagnostic on failure.
    fn parse_int(&mut self, val: &str, key: &str, loc: &SourceLocation) -> i32 {
        val.parse::<i32>().unwrap_or_else(|_| {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::InvalidInteger {
                    value: val.to_string(),
                    field: key.to_string(),
                },
                format!("invalid integer {:?} for {}", val, key),
                loc.clone(),
            ));
            0
        })
    }

    /// Parse a Go-style duration string, recording a diagnostic on failure.
    fn parse_duration(&mut self, val: &str, key: &str, loc: &SourceLocation) -> Duration {
        match Duration::parse(val) {
            Ok(d) => d,
            Err(_) => {
                self.diagnostics.push(Diagnostic::error(
                    DiagnosticKind::InvalidDuration {
                        value: val.to_string(),
                        field: key.to_string(),
                    },
                    format!("invalid duration {:?} for {}", val, key),
                    loc.clone(),
                ));
                Duration::default()
            }
        }
    }

    /// Parse a float from a string, recording a diagnostic on failure.
    fn parse_float(&mut self, val: &str, key: &str, loc: &SourceLocation) -> f64 {
        val.parse::<f64>().unwrap_or_else(|_| {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::InvalidFloat {
                    value: val.to_string(),
                    field: key.to_string(),
                },
                format!("invalid float {:?} for {}", val, key),
                loc.clone(),
            ));
            0.0
        })
    }
}

/// Apply human-specific configuration fields.
fn apply_human_field(cfg: &mut HumanConfig, key: &str, val: &str) {
    match key {
        "mode" => cfg.mode = val.to_string(),
        "default" => cfg.default = val.to_string(),
        "prompt" => cfg.prompt = val.to_string(),
        "questions_key" => cfg.questions_key = val.to_string(),
        "answers_key" => cfg.answers_key = val.to_string(),
        _ => {}
    }
}

/// Apply subgraph-specific configuration fields.
fn apply_subgraph_field(cfg: &mut SubgraphConfig, key: &str, val: &str) {
    match key {
        "ref" => cfg.ref_path = val.to_string(),
        "params" => cfg.params = parse_params_block(val),
        _ => {}
    }
}

/// Parse a raw block of key: value lines into a map.
fn parse_params_block(raw: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for line in raw.split('\n') {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((k, v)) = split_key_value(line) {
            params.insert(k, unquote_raw(&v));
        }
    }
    params
}

/// Split a comma-separated string into a list of trimmed parts.
fn split_comma(s: &str) -> Vec<String> {
    s.split(',').map(|p| p.trim().to_string()).collect()
}

/// Split "key: value" into (key, value).
fn split_key_value(line: &str) -> Option<(String, String)> {
    let idx = line.find(':')?;
    Some((
        line[..idx].trim().to_string(),
        line[idx + 1..].trim().to_string(),
    ))
}

/// Unquote a double-quoted string, handling only `\"` and `\\` escapes.
/// Go parity: dippin-lang's `unquoteRaw` does not translate `\n`/`\t`/`\r`.
fn unquote_raw(raw: &str) -> String {
    if raw.len() < 2 || !raw.starts_with('"') || !raw.ends_with('"') {
        return raw.to_string();
    }
    let inner = &raw[1..raw.len() - 1];
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some('"') => {
                    result.push('"');
                    chars.next();
                }
                Some('\\') => {
                    result.push('\\');
                    chars.next();
                }
                _ => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Parse a raw stylesheet block into rules.
fn parse_stylesheet_raw(raw: &str) -> Vec<StylesheetRule> {
    let lines: Vec<&str> = raw.split('\n').collect();
    let mut rules = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            i += 1;
            continue;
        }
        let indent = lines[i].len() - lines[i].trim_start().len();
        if indent == 0 {
            let selector = parse_selector(trimmed);
            let mut properties = HashMap::new();
            i += 1;
            while i < lines.len() {
                let line = lines[i];
                let line_trimmed = line.trim();
                if line_trimmed.is_empty() {
                    i += 1;
                    continue;
                }
                let line_indent = line.len() - line.trim_start().len();
                if line_indent == 0 {
                    break;
                }
                if let Some((k, v)) = split_key_value(line_trimmed) {
                    properties.insert(k, v);
                }
                i += 1;
            }
            rules.push(StylesheetRule {
                selector,
                properties,
            });
        } else {
            i += 1;
        }
    }
    rules
}

/// Convert a selector string to a StyleSelector.
fn parse_selector(s: &str) -> StyleSelector {
    if s == "*" {
        return StyleSelector {
            kind: "universal".to_string(),
            value: "*".to_string(),
        };
    }
    if let Some(rest) = s.strip_prefix('.') {
        return StyleSelector {
            kind: "class".to_string(),
            value: rest.to_string(),
        };
    }
    if let Some(rest) = s.strip_prefix('#') {
        return StyleSelector {
            kind: "id".to_string(),
            value: rest.to_string(),
        };
    }
    StyleSelector {
        kind: "kind".to_string(),
        value: s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_workflow() {
        let input = r#"workflow Minimal
  goal: "Test workflow"
  start: Ask
  exit: Done

  human Ask
    mode: freeform

  agent Done
    prompt:
      Complete the task.

  edges
    Ask -> Done
"#;
        let parser = Parser::new(input, "test.dip");
        let wf = parser.parse().expect("should parse");
        assert_eq!(wf.name, "Minimal");
        assert_eq!(wf.goal, "Test workflow");
        assert_eq!(wf.start, "Ask");
        assert_eq!(wf.exit, "Done");
        assert_eq!(wf.nodes.len(), 2);
        assert_eq!(wf.edges.len(), 1);

        // Check human node
        let ask = wf.nodes.iter().find(|n| n.id == "Ask").unwrap();
        assert_eq!(ask.kind, NodeKind::Human);
        match &ask.config {
            NodeConfig::Human(cfg) => assert_eq!(cfg.mode, "freeform"),
            _ => panic!("expected HumanConfig"),
        }

        // Check agent node
        let done = wf.nodes.iter().find(|n| n.id == "Done").unwrap();
        assert_eq!(done.kind, NodeKind::Agent);
        match &done.config {
            NodeConfig::Agent(cfg) => assert_eq!(cfg.prompt, "Complete the task."),
            _ => panic!("expected AgentConfig"),
        }

        // Check edge
        assert_eq!(wf.edges[0].from, "Ask");
        assert_eq!(wf.edges[0].to, "Done");
    }

    #[test]
    fn test_parse_edge_with_condition() {
        let input = r#"workflow Test
  goal: test
  start: A
  exit: B

  agent A
    prompt: do stuff

  agent B
    prompt: done

  edges
    A -> B when ctx.outcome = success label: pass
"#;
        let parser = Parser::new(input, "test.dip");
        let wf = parser.parse().expect("should parse");
        assert_eq!(wf.edges.len(), 1);
        let edge = &wf.edges[0];
        assert!(edge.condition.is_some());
        assert_eq!(edge.condition.as_ref().unwrap().raw, "ctx.outcome = success");
        assert_eq!(edge.label, "pass");
    }

    #[test]
    fn test_parse_edge_with_restart() {
        let input = r#"workflow Test
  goal: test
  start: A
  exit: B

  agent A
    prompt: do

  agent B
    prompt: done

  edges
    A -> B restart: true
"#;
        let parser = Parser::new(input, "test.dip");
        let wf = parser.parse().expect("should parse");
        assert!(wf.edges[0].restart);
    }

    #[test]
    fn test_parse_parallel_and_fan_in() {
        let input = r#"workflow Test
  goal: test
  start: S
  exit: E

  agent S
    prompt: start

  parallel P -> A, B

  agent A
    prompt: a

  agent B
    prompt: b

  fan_in J <- A, B

  agent E
    prompt: end

  edges
    S -> P
    P -> A
    P -> B
    A -> J
    B -> J
    J -> E
"#;
        let parser = Parser::new(input, "test.dip");
        let wf = parser.parse().expect("should parse");

        let p = wf.nodes.iter().find(|n| n.id == "P").unwrap();
        assert_eq!(p.kind, NodeKind::Parallel);
        match &p.config {
            NodeConfig::Parallel(cfg) => {
                assert_eq!(cfg.targets, vec!["A", "B"]);
            }
            _ => panic!("expected ParallelConfig"),
        }

        let j = wf.nodes.iter().find(|n| n.id == "J").unwrap();
        assert_eq!(j.kind, NodeKind::FanIn);
        match &j.config {
            NodeConfig::FanIn(cfg) => {
                assert_eq!(cfg.sources, vec!["A", "B"]);
            }
            _ => panic!("expected FanInConfig"),
        }
    }

    #[test]
    fn test_parse_defaults() {
        let input = r#"workflow Test
  goal: test
  start: A
  exit: A

  defaults
    max_retries: 3
    fidelity: summary:medium
    model: gpt-4

  agent A
    prompt: do

  edges
    A -> A
"#;
        let parser = Parser::new(input, "test.dip");
        let wf = parser.parse().expect("should parse");
        assert_eq!(wf.defaults.max_retries, 3);
        assert_eq!(wf.defaults.fidelity, "summary:medium");
        assert_eq!(wf.defaults.model, "gpt-4");
    }

    #[test]
    fn test_parse_tool_node() {
        let input = r#"workflow Test
  goal: test
  start: T
  exit: T

  tool T
    label: "Run Tests"
    command:
      set -eu
      echo hello

  edges
    T -> T
"#;
        let parser = Parser::new(input, "test.dip");
        let wf = parser.parse().expect("should parse");
        let t = wf.nodes.iter().find(|n| n.id == "T").unwrap();
        assert_eq!(t.kind, NodeKind::Tool);
        assert_eq!(t.label, "Run Tests");
        match &t.config {
            NodeConfig::Tool(cfg) => {
                assert_eq!(cfg.command, "set -eu\necho hello");
            }
            _ => panic!("expected ToolConfig"),
        }
    }

    #[test]
    fn test_parse_agent_with_model() {
        let input = r#"workflow Test
  goal: test
  start: A
  exit: A

  agent A
    label: "Test Agent"
    model: claude-sonnet-4-6
    provider: anthropic
    reasoning_effort: high
    goal_gate: true
    prompt:
      Do the thing.

  edges
    A -> A
"#;
        let parser = Parser::new(input, "test.dip");
        let wf = parser.parse().expect("should parse");
        let a = wf.nodes.iter().find(|n| n.id == "A").unwrap();
        match &a.config {
            NodeConfig::Agent(cfg) => {
                assert_eq!(cfg.model, "claude-sonnet-4-6");
                assert_eq!(cfg.provider, "anthropic");
                assert_eq!(cfg.reasoning_effort, "high");
                assert!(cfg.goal_gate);
                assert_eq!(cfg.prompt, "Do the thing.");
            }
            _ => panic!("expected AgentConfig"),
        }
    }

    #[test]
    fn test_unquote_raw() {
        assert_eq!(unquote_raw("hello"), "hello");
        assert_eq!(unquote_raw("\"hello\""), "hello");
        assert_eq!(unquote_raw("\"he\\\"llo\""), "he\"llo");
        assert_eq!(unquote_raw(""), "");
    }

    #[test]
    fn test_unquote_raw_only_handles_quote_and_backslash() {
        // Go's unquoteRaw only handles \" and \\
        let result = unquote_raw(r#""line1\nline2""#);
        assert_eq!(result, r"line1\nline2");
    }

    #[test]
    fn test_workflow_version_field() {
        let src = "workflow F\n  version: 1.0\n  start: A\n  exit: A\n  agent A\n    prompt: x\n    model: m\n    provider: p\n";
        let wf = crate::parse(src, "t.dip").unwrap();
        assert_eq!(wf.version, "1.0");
    }

    #[test]
    fn test_unknown_edge_attribute_is_silent() {
        // Go reference silently ignores unknown edge attributes
        let src = "workflow F\n  start: A\n  exit: B\n  agent A\n    prompt: x\n    model: m\n    provider: p\n  agent B\n    prompt: y\n    model: m\n    provider: p\n  edges\n    A -> B foo: bar\n";
        crate::parse(src, "t.dip").expect("unknown edge attr should be ignored");
    }

    #[test]
    fn test_split_comma() {
        assert_eq!(split_comma("a, b, c"), vec!["a", "b", "c"]);
        assert_eq!(split_comma("single"), vec!["single"]);
    }
}
