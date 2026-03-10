// ABOUTME: Unit tests for DotParser, covering statement parsing, source ranges, and cursor-to-node mapping.
// ABOUTME: Tests digraphs, undirected graphs, attributes, comments, quoted identifiers, and edge cases.

import XCTest
@testable import DotViewer

final class DotParserTests: XCTestCase {

    // MARK: - Basic Parsing

    func testEmptyStringProducesNoStatements() {
        let graph = DotParser.parse("")
        XCTAssertTrue(graph.statements.isEmpty)
    }

    func testWhitespaceOnlyProducesNoStatements() {
        let graph = DotParser.parse("   \n\t\n  ")
        XCTAssertTrue(graph.statements.isEmpty)
    }

    func testSimpleDigraphParseNodes() {
        let dot = """
        digraph G {
            A
            B
        }
        """
        let graph = DotParser.parse(dot)
        let nodeIds = graph.statements.compactMap {
            if case .nodeDefinition(let id, _) = $0 { return id }
            return nil
        }
        XCTAssertEqual(nodeIds, ["A", "B"])
    }

    func testSimpleEdge() {
        let dot = """
        digraph G {
            A -> B
        }
        """
        let graph = DotParser.parse(dot)
        let edges = graph.statements.compactMap {
            if case .edge(let from, let to, _, _, _) = $0 { return (from, to) }
            return nil
        }
        XCTAssertEqual(edges.count, 1)
        XCTAssertEqual(edges[0].0, "A")
        XCTAssertEqual(edges[0].1, "B")
    }

    func testUndirectedEdge() {
        let dot = """
        graph G {
            A -- B
        }
        """
        let graph = DotParser.parse(dot)
        let edges = graph.statements.compactMap {
            if case .edge(let from, let to, _, _, _) = $0 { return (from, to) }
            return nil
        }
        XCTAssertEqual(edges.count, 1)
        XCTAssertEqual(edges[0].0, "A")
        XCTAssertEqual(edges[0].1, "B")
    }

    // MARK: - Node Attributes

    func testNodeWithAttributes() {
        let dot = """
        digraph G {
            A [label="Hello" shape=box]
        }
        """
        let graph = DotParser.parse(dot)
        let nodeIds = graph.statements.compactMap {
            if case .nodeDefinition(let id, _) = $0 { return id }
            return nil
        }
        XCTAssertEqual(nodeIds, ["A"])
    }

    func testEdgeWithAttributes() {
        let dot = """
        digraph G {
            A -> B [label="edge" color=red]
        }
        """
        let graph = DotParser.parse(dot)
        let edges = graph.statements.compactMap {
            if case .edge(let from, let to, _, _, _) = $0 { return (from, to) }
            return nil
        }
        XCTAssertEqual(edges.count, 1)
        XCTAssertEqual(edges[0].0, "A")
        XCTAssertEqual(edges[0].1, "B")
    }

    // MARK: - Graph Attributes

    func testGraphAttributeStatement() {
        let dot = """
        digraph G {
            graph [rankdir=LR]
            A -> B
        }
        """
        let graph = DotParser.parse(dot)
        let hasGraphAttr = graph.statements.contains {
            if case .graphAttribute = $0 { return true }
            return false
        }
        XCTAssertTrue(hasGraphAttr)
    }

    func testNodeKeywordAsGraphAttribute() {
        let dot = """
        digraph G {
            node [shape=box]
            A
        }
        """
        let graph = DotParser.parse(dot)
        let hasGraphAttr = graph.statements.contains {
            if case .graphAttribute = $0 { return true }
            return false
        }
        XCTAssertTrue(hasGraphAttr)
        let nodeIds = graph.statements.compactMap {
            if case .nodeDefinition(let id, _) = $0 { return id }
            return nil
        }
        XCTAssertEqual(nodeIds, ["A"])
    }

    // MARK: - Quoted Identifiers

    func testQuotedNodeIdentifier() {
        let dot = """
        digraph G {
            "my node" -> "other node"
        }
        """
        let graph = DotParser.parse(dot)
        let edges = graph.statements.compactMap {
            if case .edge(let from, let to, _, _, _) = $0 { return (from, to) }
            return nil
        }
        XCTAssertEqual(edges.count, 1)
        XCTAssertEqual(edges[0].0, "my node")
        XCTAssertEqual(edges[0].1, "other node")
    }

    func testHyphenatedNodeName() {
        let dot = """
        digraph G {
            my_node -> other_node
        }
        """
        let graph = DotParser.parse(dot)
        let edges = graph.statements.compactMap {
            if case .edge(let from, let to, _, _, _) = $0 { return (from, to) }
            return nil
        }
        XCTAssertEqual(edges.count, 1)
        XCTAssertEqual(edges[0].0, "my_node")
        XCTAssertEqual(edges[0].1, "other_node")
    }

    // MARK: - Comments

    func testLineCommentIgnored() {
        let dot = """
        digraph G {
            // this is a comment
            A -> B
        }
        """
        let graph = DotParser.parse(dot)
        let edges = graph.statements.compactMap {
            if case .edge(let from, let to, _, _, _) = $0 { return (from, to) }
            return nil
        }
        XCTAssertEqual(edges.count, 1)
    }

    func testBlockCommentIgnored() {
        let dot = """
        digraph G {
            /* multi
               line comment */
            A -> B
        }
        """
        let graph = DotParser.parse(dot)
        let edges = graph.statements.compactMap {
            if case .edge(let from, let to, _, _, _) = $0 { return (from, to) }
            return nil
        }
        XCTAssertEqual(edges.count, 1)
    }

    // MARK: - Source Ranges

    func testNodeSourceRangeCoversFullStatement() {
        let dot = "digraph G {\n    A [label=\"Hi\"]\n}"
        let graph = DotParser.parse(dot)
        let nodeStmt = graph.statements.first {
            if case .nodeDefinition = $0 { return true }
            return false
        }
        XCTAssertNotNil(nodeStmt)
        let nsText = dot as NSString
        let extracted = nsText.substring(with: nodeStmt!.sourceRange)
        XCTAssertTrue(extracted.contains("A"))
        XCTAssertTrue(extracted.contains("label"))
    }

    func testEdgeSourceRangeCoversFullStatement() {
        let dot = "digraph G {\n    A -> B [color=red]\n}"
        let graph = DotParser.parse(dot)
        let edgeStmt = graph.statements.first {
            if case .edge = $0 { return true }
            return false
        }
        XCTAssertNotNil(edgeStmt)
        let nsText = dot as NSString
        let extracted = nsText.substring(with: edgeStmt!.sourceRange)
        XCTAssertTrue(extracted.contains("A"))
        XCTAssertTrue(extracted.contains("B"))
        XCTAssertTrue(extracted.contains("color"))
    }

    // MARK: - statementAt(offset:)

    func testStatementAtOffsetFindsCorrectStatement() {
        let dot = "digraph G {\n    A\n    B -> C\n}"
        let graph = DotParser.parse(dot)

        // Find offset of "B" in the source
        let bOffset = (dot as NSString).range(of: "B").location
        let stmt = graph.statementAt(offset: bOffset)
        XCTAssertNotNil(stmt)
        if case .edge(let from, _, _, _, _) = stmt! {
            XCTAssertEqual(from, "B")
        } else {
            XCTFail("Expected edge statement at B's offset")
        }
    }

    func testStatementAtOffsetReturnsNilOutsideStatements() {
        let dot = "digraph G {\n\n\n    A\n}"
        let graph = DotParser.parse(dot)
        // Offset in blank line area should return nil
        let stmt = graph.statementAt(offset: 13)
        XCTAssertNil(stmt)
    }

    // MARK: - nodeIdAt(offset:)

    func testNodeIdAtOffsetForNodeDefinition() {
        let dot = "digraph G {\n    A [label=\"Hello\"]\n}"
        let graph = DotParser.parse(dot)
        let aOffset = (dot as NSString).range(of: "A").location
        let stmt = graph.statementAt(offset: aOffset)
        XCTAssertNotNil(stmt)
        XCTAssertEqual(stmt?.nodeIdAt(offset: aOffset), "A")
    }

    func testNodeIdAtOffsetInAttributeAreaStillReturnsNode() {
        let dot = "digraph G {\n    A [label=\"Hello\"]\n}"
        let graph = DotParser.parse(dot)
        let labelOffset = (dot as NSString).range(of: "label").location
        let stmt = graph.statementAt(offset: labelOffset)
        XCTAssertNotNil(stmt)
        XCTAssertEqual(stmt?.nodeIdAt(offset: labelOffset), "A")
    }

    func testNodeIdAtOffsetForEdgeSelectsCloserNode() {
        let dot = "digraph G {\n    A -> B\n}"
        let graph = DotParser.parse(dot)

        // Offset near A should return A
        let aOffset = (dot as NSString).range(of: "A", options: [], range: NSRange(location: 12, length: 10)).location
        let stmtA = graph.statementAt(offset: aOffset)
        XCTAssertEqual(stmtA?.nodeIdAt(offset: aOffset), "A")

        // Offset near B should return B
        let bOffset = (dot as NSString).range(of: "B", options: [], range: NSRange(location: 12, length: 10)).location
        let stmtB = graph.statementAt(offset: bOffset)
        XCTAssertEqual(stmtB?.nodeIdAt(offset: bOffset), "B")
    }

    // MARK: - definitionForNode(_:)

    func testDefinitionForNodeFindsNodeDefinition() {
        let dot = """
        digraph G {
            A [label="Hello"]
            A -> B
        }
        """
        let graph = DotParser.parse(dot)
        let stmt = graph.definitionForNode("A")
        XCTAssertNotNil(stmt)
        if case .nodeDefinition(let id, _) = stmt! {
            XCTAssertEqual(id, "A")
        } else {
            XCTFail("Expected node definition, got edge")
        }
    }

    func testDefinitionForNodeFallsBackToEdge() {
        let dot = """
        digraph G {
            A -> B
        }
        """
        let graph = DotParser.parse(dot)
        // B has no explicit definition, should fall back to the edge
        let stmt = graph.definitionForNode("B")
        XCTAssertNotNil(stmt)
        if case .edge(_, let to, _, _, _) = stmt! {
            XCTAssertEqual(to, "B")
        } else {
            XCTFail("Expected edge fallback for undefined node")
        }
    }

    func testDefinitionForNodeReturnsNilForUnknown() {
        let dot = """
        digraph G {
            A -> B
        }
        """
        let graph = DotParser.parse(dot)
        XCTAssertNil(graph.definitionForNode("Z"))
    }

    // MARK: - Multiple Edges and Complex Graphs

    func testMultipleEdges() {
        let dot = """
        digraph G {
            A -> B
            B -> C
            C -> A
        }
        """
        let graph = DotParser.parse(dot)
        let edges = graph.statements.compactMap {
            if case .edge(let from, let to, _, _, _) = $0 { return (from, to) }
            return nil
        }
        XCTAssertEqual(edges.count, 3)
    }

    func testMixedNodesAndEdges() {
        let dot = """
        digraph G {
            A [shape=box]
            B [shape=circle]
            A -> B [label="connects"]
        }
        """
        let graph = DotParser.parse(dot)
        let nodes = graph.statements.compactMap {
            if case .nodeDefinition(let id, _) = $0 { return id }
            return nil
        }
        let edges = graph.statements.compactMap {
            if case .edge(let from, let to, _, _, _) = $0 { return (from, to) }
            return nil
        }
        XCTAssertEqual(nodes, ["A", "B"])
        XCTAssertEqual(edges.count, 1)
    }

    func testSemicolonSeparatedStatements() {
        let dot = "digraph G { A; B; A -> B }"
        let graph = DotParser.parse(dot)
        let nodeIds = graph.statements.compactMap {
            if case .nodeDefinition(let id, _) = $0 { return id }
            return nil
        }
        XCTAssertTrue(nodeIds.contains("A"))
        XCTAssertTrue(nodeIds.contains("B"))
    }

    // MARK: - Keywords Not Treated as Nodes

    func testDigraphKeywordNotParsedAsNode() {
        let dot = "digraph G { A }"
        let graph = DotParser.parse(dot)
        let nodeIds = graph.statements.compactMap {
            if case .nodeDefinition(let id, _) = $0 { return id }
            return nil
        }
        XCTAssertFalse(nodeIds.contains("digraph"))
        XCTAssertFalse(nodeIds.contains("G"))
        XCTAssertTrue(nodeIds.contains("A"))
    }

    func testStrictKeywordIgnored() {
        let dot = "strict digraph G { A -> B }"
        let graph = DotParser.parse(dot)
        let nodeIds = graph.statements.compactMap {
            if case .nodeDefinition(let id, _) = $0 { return id }
            return nil
        }
        XCTAssertFalse(nodeIds.contains("strict"))
    }

    // MARK: - Graph Attribute for graphAttribute

    func testGraphAttributeReturnsNilNodeId() {
        let dot = "digraph G { graph [rankdir=LR] }"
        let graph = DotParser.parse(dot)
        let graphAttr = graph.statements.first {
            if case .graphAttribute = $0 { return true }
            return false
        }
        XCTAssertNotNil(graphAttr)
        XCTAssertNil(graphAttr?.nodeIdAt(offset: graphAttr!.sourceRange.location))
    }
}
