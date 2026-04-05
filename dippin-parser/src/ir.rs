// ABOUTME: Intermediate representation types for Dippin workflows.
// ABOUTME: Defines the canonical data model shared between parsing and export.

use std::collections::HashMap;

/// Workflow is the top-level IR structure representing a complete pipeline.
#[derive(Debug, Clone, Default)]
pub struct Workflow {
    pub name: String,
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
pub struct WorkflowDefaults {
    pub model: String,
    pub provider: String,
    pub retry_policy: String,
    pub max_retries: i32,
    pub fidelity: String,
    pub max_restarts: i32,
    pub restart_target: String,
    pub cache_tools: bool,
    pub compaction: String,
    pub on_resume: String,
}

/// Node represents a single step in the workflow.
#[derive(Debug, Clone)]
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
pub enum NodeKind {
    Agent,
    Human,
    Tool,
    Parallel,
    FanIn,
    Subgraph,
}

impl NodeKind {
    /// Parse a node kind from a string keyword.
    pub fn from_str(s: &str) -> Option<NodeKind> {
        match s {
            "agent" => Some(NodeKind::Agent),
            "human" => Some(NodeKind::Human),
            "tool" => Some(NodeKind::Tool),
            "parallel" => Some(NodeKind::Parallel),
            "fan_in" => Some(NodeKind::FanIn),
            "subgraph" => Some(NodeKind::Subgraph),
            _ => None,
        }
    }
}

/// NodeConfig holds kind-specific configuration for a node.
#[derive(Debug, Clone)]
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
pub struct AgentConfig {
    pub prompt: String,
    pub system_prompt: String,
    pub model: String,
    pub provider: String,
    pub max_turns: i32,
    pub cmd_timeout: String,
    pub cache_tools: bool,
    pub compaction: String,
    pub compaction_threshold: f64,
    pub reasoning_effort: String,
    pub fidelity: String,
    pub auto_status: bool,
    pub goal_gate: bool,
    pub response_format: String,
    pub response_schema: String,
    pub params: HashMap<String, String>,
}

/// HumanConfig holds configuration for human gate nodes.
#[derive(Debug, Clone, Default)]
pub struct HumanConfig {
    pub mode: String,
    pub default: String,
    pub prompt: String,
    pub questions_key: String,
    pub answers_key: String,
}

/// ToolConfig holds configuration for shell command nodes.
#[derive(Debug, Clone, Default)]
pub struct ToolConfig {
    pub command: String,
    pub timeout: String,
    pub outputs: Vec<String>,
}

/// ParallelConfig holds configuration for fan-out nodes.
#[derive(Debug, Clone, Default)]
pub struct ParallelConfig {
    pub targets: Vec<String>,
    pub branches: Vec<BranchConfig>,
}

/// BranchConfig holds per-branch configuration for block-form parallel nodes.
#[derive(Debug, Clone, Default)]
pub struct BranchConfig {
    pub target: String,
    pub model: String,
    pub provider: String,
    pub fidelity: String,
}

/// FanInConfig holds configuration for join nodes.
#[derive(Debug, Clone, Default)]
pub struct FanInConfig {
    pub sources: Vec<String>,
}

/// SubgraphConfig holds configuration for embedded sub-pipeline nodes.
#[derive(Debug, Clone, Default)]
pub struct SubgraphConfig {
    pub ref_path: String,
    pub params: HashMap<String, String>,
}

/// RetryConfig specifies retry behavior for a node.
#[derive(Debug, Clone, Default)]
pub struct RetryConfig {
    pub policy: String,
    pub max_retries: i32,
    pub base_delay: String,
    pub retry_target: String,
    pub fallback_target: String,
}

/// NodeIO declares what context keys a node reads and writes.
#[derive(Debug, Clone, Default)]
pub struct NodeIO {
    pub reads: Vec<String>,
    pub writes: Vec<String>,
}

/// SourceLocation identifies a position in the source file.
#[derive(Debug, Clone, Default)]
pub struct SourceLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

/// Edge represents a connection between nodes in the workflow graph.
#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub condition: Option<Condition>,
    pub weight: i32,
    pub restart: bool,
    pub source: SourceLocation,
}

/// Condition is a parsed, validated boolean expression attached to an edge.
#[derive(Debug, Clone)]
pub struct Condition {
    pub raw: String,
    pub parsed: Option<ConditionExpr>,
}

/// ConditionExpr is the AST for edge conditions.
#[derive(Debug, Clone)]
pub enum ConditionExpr {
    Compare {
        variable: String,
        op: String,
        value: String,
    },
    And {
        left: Box<ConditionExpr>,
        right: Box<ConditionExpr>,
    },
    Or {
        left: Box<ConditionExpr>,
        right: Box<ConditionExpr>,
    },
    Not {
        inner: Box<ConditionExpr>,
    },
}

/// StylesheetRule pairs a selector with a set of properties.
#[derive(Debug, Clone)]
pub struct StylesheetRule {
    pub selector: StyleSelector,
    pub properties: HashMap<String, String>,
}

/// StyleSelector identifies what a stylesheet rule targets.
#[derive(Debug, Clone)]
pub struct StyleSelector {
    pub kind: String,
    pub value: String,
}

/// Create a default NodeConfig for a given NodeKind.
pub fn default_node_config(kind: &NodeKind) -> NodeConfig {
    match kind {
        NodeKind::Agent => NodeConfig::Agent(AgentConfig {
            params: HashMap::new(),
            ..Default::default()
        }),
        NodeKind::Human => NodeConfig::Human(HumanConfig::default()),
        NodeKind::Tool => NodeConfig::Tool(ToolConfig::default()),
        NodeKind::Parallel => NodeConfig::Parallel(ParallelConfig::default()),
        NodeKind::FanIn => NodeConfig::FanIn(FanInConfig::default()),
        NodeKind::Subgraph => NodeConfig::Subgraph(SubgraphConfig {
            params: HashMap::new(),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_kind_from_str() {
        assert_eq!(NodeKind::from_str("agent"), Some(NodeKind::Agent));
        assert_eq!(NodeKind::from_str("human"), Some(NodeKind::Human));
        assert_eq!(NodeKind::from_str("tool"), Some(NodeKind::Tool));
        assert_eq!(NodeKind::from_str("parallel"), Some(NodeKind::Parallel));
        assert_eq!(NodeKind::from_str("fan_in"), Some(NodeKind::FanIn));
        assert_eq!(NodeKind::from_str("subgraph"), Some(NodeKind::Subgraph));
        assert_eq!(NodeKind::from_str("unknown"), None);
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
