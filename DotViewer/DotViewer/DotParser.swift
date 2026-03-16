// ABOUTME: Bridges the Rust DOT parser (via UniFFI) to the Swift app layer with NSRange-based APIs.
// ABOUTME: Provides extensions on generated types for cursor-to-node detection and source range mapping.

import Foundation

// MARK: - DotStatement Extensions

extension DotStatement {

    /// The source range of this statement as an NSRange, for use with NSString and NSTextView APIs.
    var sourceRange: NSRange {
        switch self {
        case .nodeDefinition(_, let range):
            return NSRange(location: Int(range.location), length: Int(range.length))
        case .edge(_, _, let range, _, _):
            return NSRange(location: Int(range.location), length: Int(range.length))
        case .graphAttribute(let range):
            return NSRange(location: Int(range.location), length: Int(range.length))
        }
    }

    /// Returns the node ID relevant to a given cursor offset within this statement.
    /// For node definitions, always returns the node ID.
    /// For edges, returns whichever node the cursor is closest to.
    func nodeIdAt(offset: Int) -> String? {
        return DotViewer.nodeIdAt(statement: self, offset: UInt32(offset))
    }
}

// MARK: - DotGraph Extensions

extension DotGraph {

    /// Find the statement containing the given character offset
    func statementAt(offset: Int) -> DotStatement? {
        return DotViewer.statementAt(graph: self, offset: UInt32(offset))
    }

    /// Find the first node definition for a given node ID, falling back to any
    /// statement that references it (edge from/to).
    func definitionForNode(_ nodeId: String) -> DotStatement? {
        return DotViewer.definitionForNode(graph: self, nodeId: nodeId)
    }
}

// MARK: - DotParser

struct DotParser {

    /// Parse DOT source text into a structured graph model with source ranges.
    /// Delegates to the Rust parser via UniFFI.
    static func parse(_ text: String) -> DotGraph {
        return parseDot(source: text)
    }
}
