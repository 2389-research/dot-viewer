// ABOUTME: Intermediate representation types for Dippin workflows.
// ABOUTME: Defines the canonical data model shared between parsing and export.

use std::str::FromStr;
use std::sync::Arc;

use indexmap::IndexMap;

use crate::duration::Duration;

/// Workflow is the top-level IR structure representing a complete pipeline.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Workflow {
    /// Workflow identifier (used as the DOT graph name).
    pub name: String,
    /// Optional schema/author version string.
    pub version: String,
    /// Free-form description of what this workflow accomplishes.
    pub goal: String,
    /// ID of the entry node (must reference a declared Node).
    pub start: String,
    /// ID of the terminal node (must reference a declared Node).
    pub exit: String,
    /// Workflow-wide defaults applied to nodes that do not override them.
    pub defaults: WorkflowDefaults,
    /// All declared nodes, in source order.
    pub nodes: Vec<Node>,
    /// All declared edges, in source order.
    pub edges: Vec<Edge>,
    /// Stylesheet rules applied during DOT export.
    pub stylesheet: Vec<StylesheetRule>,
}

/// WorkflowDefaults holds graph-level configuration that applies to all nodes
/// unless overridden at the node level.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct WorkflowDefaults {
    /// Default LLM model identifier (e.g., `claude-sonnet-4-6`).
    pub model: String,
    /// Default LLM provider identifier (e.g., `anthropic`, `openai`).
    pub provider: String,
    /// Default retry policy name (e.g., `exponential`, `fixed`, `none`).
    pub retry_policy: String,
    /// Default maximum retry attempts per node.
    pub max_retries: u32,
    /// Default information-fidelity setting (e.g., `summary:medium`).
    pub fidelity: String,
    /// Maximum number of times the workflow may be restarted.
    pub max_restarts: u32,
    /// Default node ID to restart from when a restart edge fires.
    pub restart_target: String,
    /// Whether tool results are cached across runs by default.
    pub cache_tools: bool,
    /// Default context-compaction strategy name.
    pub compaction: String,
    /// Default action to take when a workflow is resumed mid-flight.
    pub on_resume: String,
}

/// Node represents a single step in the workflow.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Node {
    /// Unique node identifier as written in the source.
    pub id: String,
    /// Discriminator for the node category (agent, human, tool, etc.).
    pub kind: NodeKind,
    /// Optional human-readable label rendered in the DOT output.
    pub label: String,
    /// Stylesheet class names applied to this node.
    pub classes: Vec<String>,
    /// Kind-specific configuration payload.
    pub config: NodeConfig,
    /// Retry behavior for this node.
    pub retry: RetryConfig,
    /// Declared context reads/writes for this node.
    pub io: NodeIO,
    /// Source position where this node was declared.
    pub source: SourceLocation,
}

/// NodeKind enumerates node types explicitly.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NodeKind {
    Agent,
    Human,
    Tool,
    Parallel,
    FanIn,
    Subgraph,
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            NodeKind::Agent => "agent",
            NodeKind::Human => "human",
            NodeKind::Tool => "tool",
            NodeKind::Parallel => "parallel",
            NodeKind::FanIn => "fan_in",
            NodeKind::Subgraph => "subgraph",
        })
    }
}

impl FromStr for NodeKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "agent" => Ok(NodeKind::Agent),
            "human" => Ok(NodeKind::Human),
            "tool" => Ok(NodeKind::Tool),
            "parallel" => Ok(NodeKind::Parallel),
            "fan_in" => Ok(NodeKind::FanIn),
            "subgraph" => Ok(NodeKind::Subgraph),
            _ => Err(format!("unknown node kind: {}", s)),
        }
    }
}

/// NodeConfig holds kind-specific configuration for a node.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum NodeConfig {
    Agent(AgentConfig),
    Human(HumanConfig),
    Tool(ToolConfig),
    Parallel(ParallelConfig),
    FanIn(FanInConfig),
    Subgraph(SubgraphConfig),
}

/// AgentConfig holds configuration for LLM agent nodes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct AgentConfig {
    /// User prompt template sent to the model on each turn.
    pub prompt: String,
    /// System prompt establishing the agent's persona and rules.
    pub system_prompt: String,
    /// LLM model identifier (overrides workflow defaults).
    pub model: String,
    /// LLM provider identifier (overrides workflow defaults).
    pub provider: String,
    /// Maximum reasoning/tool-call turns before giving up.
    pub max_turns: u32,
    /// Maximum wall-clock time per agent invocation.
    pub cmd_timeout: Duration,
    /// Whether tool results are cached across runs.
    pub cache_tools: bool,
    /// Context-compaction strategy name.
    pub compaction: String,
    /// Token-count fraction at which context compaction triggers (0.0–1.0).
    pub compaction_threshold: f64,
    /// Reasoning-effort hint for models that support it (e.g., `low`, `high`).
    pub reasoning_effort: String,
    /// Information-fidelity setting (e.g., `summary:medium`).
    pub fidelity: String,
    /// Whether the agent auto-emits status updates between turns.
    pub auto_status: bool,
    /// If true, the workflow goal must be re-checked after this agent runs.
    pub goal_gate: bool,
    /// Desired response format identifier (e.g., `json`, `text`).
    pub response_format: String,
    /// JSON schema enforced on structured responses.
    pub response_schema: String,
    /// Free-form key/value parameters forwarded to the runtime.
    pub params: IndexMap<String, String>,
}

/// HumanConfig holds configuration for human gate nodes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct HumanConfig {
    /// Interaction mode (e.g., `freeform`, `multiple_choice`, `confirm`).
    pub mode: String,
    /// Default response used when no human input is supplied.
    pub default: String,
    /// Prompt shown to the human operator.
    pub prompt: String,
    /// Context key from which to read the questions list.
    pub questions_key: String,
    /// Context key under which collected answers are written.
    pub answers_key: String,
}

/// ToolConfig holds configuration for shell command nodes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ToolConfig {
    /// Shell command line to execute.
    pub command: String,
    /// Maximum wall-clock duration before the command is killed.
    pub timeout: Duration,
    /// Names of context keys that capture the command's outputs.
    pub outputs: Vec<String>,
}

/// ParallelConfig holds configuration for fan-out nodes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParallelConfig {
    /// Inline form: target node IDs to fan out to.
    pub targets: Vec<String>,
    /// Block form: per-branch configuration overrides.
    pub branches: Vec<BranchConfig>,
}

/// BranchConfig holds per-branch configuration for block-form parallel nodes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct BranchConfig {
    /// Node ID this branch fans out to.
    pub target: String,
    /// Per-branch model override.
    pub model: String,
    /// Per-branch provider override.
    pub provider: String,
    /// Per-branch fidelity override.
    pub fidelity: String,
}

/// FanInConfig holds configuration for join nodes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct FanInConfig {
    /// Node IDs whose outputs are joined here.
    pub sources: Vec<String>,
}

/// SubgraphConfig holds configuration for embedded sub-pipeline nodes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SubgraphConfig {
    /// Path or reference identifier of the embedded sub-pipeline.
    pub ref_path: String,
    /// Parameter overrides forwarded to the sub-pipeline.
    pub params: IndexMap<String, String>,
}

/// RetryConfig specifies retry behavior for a node.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RetryConfig {
    /// Retry policy name (e.g., `exponential`, `fixed`, `none`).
    pub policy: String,
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Base delay between retries (multiplied by policy backoff).
    pub base_delay: Duration,
    /// Optional node ID to jump to when retries are exhausted.
    pub retry_target: String,
    /// Optional node ID to jump to on terminal failure.
    pub fallback_target: String,
}

/// NodeIO declares what context keys a node reads and writes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct NodeIO {
    /// Context keys this node reads from.
    pub reads: Vec<String>,
    /// Context keys this node writes to.
    pub writes: Vec<String>,
}

/// SourceLocation identifies a position in the source file.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceLocation {
    /// Source file path (shared via Arc to avoid per-token allocation).
    pub file: Arc<str>,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number (counted in characters, not bytes).
    pub column: usize,
}

impl Default for SourceLocation {
    fn default() -> Self {
        Self {
            file: Arc::from(""),
            line: 0,
            column: 0,
        }
    }
}

// Manual serde impls because `Arc<str>` does not implement `Deserialize` without
// serde's `rc` feature; we round-trip the file path as a borrowed `&str`.
#[cfg(feature = "serde")]
impl serde::Serialize for SourceLocation {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("SourceLocation", 3)?;
        s.serialize_field("file", &*self.file)?;
        s.serialize_field("line", &self.line)?;
        s.serialize_field("column", &self.column)?;
        s.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for SourceLocation {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Helper {
            file: String,
            line: usize,
            column: usize,
        }
        let h = Helper::deserialize(deserializer)?;
        Ok(SourceLocation {
            file: Arc::from(h.file),
            line: h.line,
            column: h.column,
        })
    }
}

/// Edge represents a connection between nodes in the workflow graph.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Edge {
    /// Source node ID.
    pub from: String,
    /// Destination node ID.
    pub to: String,
    /// Optional human-readable edge label.
    pub label: String,
    /// Optional guard condition that must hold for the edge to fire.
    pub condition: Option<Condition>,
    /// Layout hint for Graphviz; higher values pull endpoints closer.
    pub weight: u32,
    /// If true, this edge restarts the workflow from the source node.
    pub restart: bool,
    /// Source position where this edge was declared.
    pub source: SourceLocation,
}

/// Condition is a raw boolean expression attached to an edge.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Condition {
    /// Original textual form of the boolean expression.
    pub raw: String,
}

/// StylesheetRule pairs a selector with a set of properties.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StylesheetRule {
    /// Selector that determines which nodes this rule targets.
    pub selector: StyleSelector,
    /// Graphviz attribute name/value pairs to apply.
    pub properties: IndexMap<String, String>,
}

/// StyleSelector identifies what a stylesheet rule targets.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StyleSelector {
    Universal,
    Class(String),
    Id(String),
    Kind(String),
}

impl NodeConfig {
    /// Create a default NodeConfig variant for a given NodeKind.
    pub fn default_for(kind: &NodeKind) -> Self {
        match kind {
            NodeKind::Agent => NodeConfig::Agent(AgentConfig::default()),
            NodeKind::Human => NodeConfig::Human(HumanConfig::default()),
            NodeKind::Tool => NodeConfig::Tool(ToolConfig::default()),
            NodeKind::Parallel => NodeConfig::Parallel(ParallelConfig::default()),
            NodeKind::FanIn => NodeConfig::FanIn(FanInConfig::default()),
            NodeKind::Subgraph => NodeConfig::Subgraph(SubgraphConfig::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_kind_from_str() {
        assert_eq!("agent".parse::<NodeKind>(), Ok(NodeKind::Agent));
        assert_eq!("human".parse::<NodeKind>(), Ok(NodeKind::Human));
        assert_eq!("tool".parse::<NodeKind>(), Ok(NodeKind::Tool));
        assert_eq!("parallel".parse::<NodeKind>(), Ok(NodeKind::Parallel));
        assert_eq!("fan_in".parse::<NodeKind>(), Ok(NodeKind::FanIn));
        assert_eq!("subgraph".parse::<NodeKind>(), Ok(NodeKind::Subgraph));
        assert!("unknown".parse::<NodeKind>().is_err());
    }

    #[test]
    fn test_node_kind_display_fromstr_roundtrip() {
        use std::str::FromStr;
        for k in &[
            NodeKind::Agent,
            NodeKind::Human,
            NodeKind::Tool,
            NodeKind::Parallel,
            NodeKind::FanIn,
            NodeKind::Subgraph,
        ] {
            let s = k.to_string();
            assert_eq!(NodeKind::from_str(&s).unwrap(), *k);
        }
    }

    #[test]
    fn test_default_node_config() {
        match NodeConfig::default_for(&NodeKind::Agent) {
            NodeConfig::Agent(cfg) => assert!(cfg.params.is_empty()),
            _ => panic!("expected AgentConfig"),
        }
        match NodeConfig::default_for(&NodeKind::Human) {
            NodeConfig::Human(_) => {}
            _ => panic!("expected HumanConfig"),
        }
        match NodeConfig::default_for(&NodeKind::Tool) {
            NodeConfig::Tool(_) => {}
            _ => panic!("expected ToolConfig"),
        }
    }
}
