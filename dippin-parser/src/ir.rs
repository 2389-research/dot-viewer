// ABOUTME: Intermediate representation types for Dippin workflows.
// ABOUTME: Defines the canonical data model shared between parsing and export.

use std::str::FromStr;

use indexmap::IndexMap;

use crate::duration::Duration;

/// Workflow is the top-level IR structure representing a complete pipeline.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct Workflow {
    pub name: String,
    pub version: String,
    pub goal: String,
    pub start: String,
    pub exit: String,
    pub defaults: WorkflowDefaults,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub stylesheet: Vec<StylesheetRule>,
}

/// WorkflowDefaults holds graph-level configuration that applies to all nodes
/// unless overridden at the node level.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct WorkflowDefaults {
    pub model: String,
    pub provider: String,
    pub retry_policy: String,
    pub max_retries: u32,
    pub fidelity: String,
    pub max_restarts: u32,
    pub restart_target: String,
    pub cache_tools: bool,
    pub compaction: String,
    pub on_resume: String,
}

/// Node represents a single step in the workflow.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    pub classes: Vec<String>,
    pub config: NodeConfig,
    pub retry: RetryConfig,
    pub io: NodeIO,
    pub source: SourceLocation,
}

/// NodeKind enumerates node types explicitly.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NodeKind {
    Agent,
    Human,
    Tool,
    Parallel,
    FanIn,
    Subgraph,
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct AgentConfig {
    pub prompt: String,
    pub system_prompt: String,
    pub model: String,
    pub provider: String,
    pub max_turns: u32,
    pub cmd_timeout: Duration,
    pub cache_tools: bool,
    pub compaction: String,
    pub compaction_threshold: f64,
    pub reasoning_effort: String,
    pub fidelity: String,
    pub auto_status: bool,
    pub goal_gate: bool,
    pub response_format: String,
    pub response_schema: String,
    pub params: IndexMap<String, String>,
}

/// HumanConfig holds configuration for human gate nodes.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct HumanConfig {
    pub mode: String,
    pub default: String,
    pub prompt: String,
    pub questions_key: String,
    pub answers_key: String,
}

/// ToolConfig holds configuration for shell command nodes.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ToolConfig {
    pub command: String,
    pub timeout: Duration,
    pub outputs: Vec<String>,
}

/// ParallelConfig holds configuration for fan-out nodes.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct ParallelConfig {
    pub targets: Vec<String>,
    pub branches: Vec<BranchConfig>,
}

/// BranchConfig holds per-branch configuration for block-form parallel nodes.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct BranchConfig {
    pub target: String,
    pub model: String,
    pub provider: String,
    pub fidelity: String,
}

/// FanInConfig holds configuration for join nodes.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct FanInConfig {
    pub sources: Vec<String>,
}

/// SubgraphConfig holds configuration for embedded sub-pipeline nodes.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SubgraphConfig {
    pub ref_path: String,
    pub params: IndexMap<String, String>,
}

/// RetryConfig specifies retry behavior for a node.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RetryConfig {
    pub policy: String,
    pub max_retries: u32,
    pub base_delay: Duration,
    pub retry_target: String,
    pub fallback_target: String,
}

/// NodeIO declares what context keys a node reads and writes.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct NodeIO {
    pub reads: Vec<String>,
    pub writes: Vec<String>,
}

/// SourceLocation identifies a position in the source file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

/// Edge represents a connection between nodes in the workflow graph.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub condition: Option<Condition>,
    pub weight: u32,
    pub restart: bool,
    pub source: SourceLocation,
}

/// Condition is a raw boolean expression attached to an edge.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Condition {
    pub raw: String,
}

/// StylesheetRule pairs a selector with a set of properties.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StylesheetRule {
    pub selector: StyleSelector,
    pub properties: IndexMap<String, String>,
}

/// StyleSelector identifies what a stylesheet rule targets.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StyleSelector {
    Universal,
    Class(String),
    Id(String),
    Kind(String),
}

/// Create a default NodeConfig for a given NodeKind.
pub fn default_node_config(kind: &NodeKind) -> NodeConfig {
    match kind {
        NodeKind::Agent => NodeConfig::Agent(AgentConfig {
            params: IndexMap::new(),
            ..Default::default()
        }),
        NodeKind::Human => NodeConfig::Human(HumanConfig::default()),
        NodeKind::Tool => NodeConfig::Tool(ToolConfig::default()),
        NodeKind::Parallel => NodeConfig::Parallel(ParallelConfig::default()),
        NodeKind::FanIn => NodeConfig::FanIn(FanInConfig::default()),
        NodeKind::Subgraph => NodeConfig::Subgraph(SubgraphConfig {
            params: IndexMap::new(),
            ..Default::default()
        }),
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
    fn test_default_node_config() {
        match default_node_config(&NodeKind::Agent) {
            NodeConfig::Agent(cfg) => assert!(cfg.params.is_empty()),
            _ => panic!("expected AgentConfig"),
        }
        match default_node_config(&NodeKind::Human) {
            NodeConfig::Human(_) => {}
            _ => panic!("expected HumanConfig"),
        }
        match default_node_config(&NodeKind::Tool) {
            NodeConfig::Tool(_) => {}
            _ => panic!("expected ToolConfig"),
        }
    }
}
