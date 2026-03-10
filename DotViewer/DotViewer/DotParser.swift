// ABOUTME: Lightweight DOT language parser that produces a structured model with source ranges.
// ABOUTME: Used for bidirectional mapping between editor cursor positions and graph elements.

import Foundation

enum DotStatement {
    case nodeDefinition(id: String, sourceRange: NSRange)
    case edge(from: String, to: String, sourceRange: NSRange, fromRange: NSRange, toRange: NSRange)
    case graphAttribute(sourceRange: NSRange)

    var sourceRange: NSRange {
        switch self {
        case .nodeDefinition(_, let range):
            return range
        case .edge(_, _, let range, _, _):
            return range
        case .graphAttribute(let range):
            return range
        }
    }

    /// Returns the node ID relevant to a given cursor offset within this statement.
    /// For node definitions, always returns the node ID.
    /// For edges, returns whichever node the cursor is closest to.
    func nodeIdAt(offset: Int) -> String? {
        switch self {
        case .nodeDefinition(let id, _):
            return id
        case .edge(let from, let to, _, let fromRange, let toRange):
            let distToFrom = abs(offset - (fromRange.location + fromRange.length / 2))
            let distToTo = abs(offset - (toRange.location + toRange.length / 2))
            return distToTo < distToFrom ? to : from
        case .graphAttribute:
            return nil
        }
    }
}

struct DotGraph {
    let statements: [DotStatement]

    /// Find the statement containing the given character offset
    func statementAt(offset: Int) -> DotStatement? {
        statements.first { NSLocationInRange(offset, $0.sourceRange) }
    }

    /// Find the first node definition for a given node ID, falling back to any
    /// statement that references it (edge from/to).
    func definitionForNode(_ nodeId: String) -> DotStatement? {
        // Priority 1: explicit node definition
        for stmt in statements {
            if case .nodeDefinition(let id, _) = stmt, id == nodeId {
                return stmt
            }
        }
        // Priority 2: first edge referencing this node
        for stmt in statements {
            if case .edge(let from, let to, _, _, _) = stmt {
                if from == nodeId || to == nodeId {
                    return stmt
                }
            }
        }
        return nil
    }
}

struct DotParser {

    /// Parse DOT source text into a structured graph model with source ranges.
    /// Single-pass state machine that tracks strings, comments, braces, and brackets.
    static func parse(_ text: String) -> DotGraph {
        let nsText = text as NSString
        let length = nsText.length
        var statements: [DotStatement] = []

        // Global keywords that are never node identifiers
        let globalKeywords: Set<String> = [
            "digraph", "graph", "subgraph", "node", "edge", "strict"
        ]

        var i = 0

        while i < length {
            // Skip whitespace and semicolons between statements
            i = skipWhitespaceAndSemicolons(in: nsText, from: i, length: length)
            guard i < length else { break }

            let ch = nsText.character(at: i)

            // Skip comments
            if ch == forwardSlash && i + 1 < length {
                let next = nsText.character(at: i + 1)
                if next == forwardSlash {
                    // Line comment — skip to end of line
                    i = skipToEndOfLine(in: nsText, from: i, length: length)
                    continue
                } else if next == asterisk {
                    // Block comment — skip to closing */
                    i = skipBlockComment(in: nsText, from: i, length: length)
                    continue
                }
            }

            // Opening/closing braces — skip them as statement boundaries
            if ch == openBrace || ch == closeBrace {
                i += 1
                continue
            }

            // Try to parse a statement starting at this position
            let stmtStart = i

            // Extract the first identifier (or skip if not an identifier character)
            guard isIdentChar(nsText.character(at: i)) || nsText.character(at: i) == doubleQuote else {
                // Not a statement start we recognize — skip to next boundary
                i = skipToStatementBoundary(in: nsText, from: i, length: length)
                continue
            }

            let (firstId, firstIdRange, afterFirst) = extractIdentifier(in: nsText, from: i, length: length)
            guard let firstId = firstId else {
                i = skipToStatementBoundary(in: nsText, from: i, length: length)
                continue
            }

            // Check if this is a keyword
            if globalKeywords.contains(firstId.lowercased()) {
                // Skip the rest of this keyword statement (e.g. `digraph G {` or `graph [rankdir=LR]`)
                let keywordLower = firstId.lowercased()
                if keywordLower == "graph" || keywordLower == "node" || keywordLower == "edge" {
                    // These can have attribute lists: `graph [rankdir=LR]`
                    let stmtEnd = findStatementEnd(in: nsText, from: stmtStart, length: length)
                    let range = NSRange(location: stmtStart, length: stmtEnd - stmtStart)
                    statements.append(.graphAttribute(sourceRange: range))
                    i = stmtEnd
                } else {
                    // digraph, subgraph, strict — skip to the brace or end
                    i = skipToStatementBoundary(in: nsText, from: afterFirst, length: length)
                }
                continue
            }

            // We have a non-keyword identifier. Scan ahead to classify the statement.
            var scanPos = afterFirst
            scanPos = skipWhitespaceOnly(in: nsText, from: scanPos, length: length)

            // Check for edge operator (-> or --)
            if scanPos + 1 < length {
                let c1 = nsText.character(at: scanPos)
                let c2 = nsText.character(at: scanPos + 1)
                let isArrow = (c1 == dash && c2 == greaterThan) || (c1 == dash && c2 == dash)
                if isArrow {
                    // Edge statement
                    let afterArrow = scanPos + 2
                    let postArrow = skipWhitespaceOnly(in: nsText, from: afterArrow, length: length)
                    let (secondId, secondIdRange, _) = extractIdentifier(in: nsText, from: postArrow, length: length)

                    let stmtEnd = findStatementEnd(in: nsText, from: stmtStart, length: length)
                    let range = NSRange(location: stmtStart, length: stmtEnd - stmtStart)

                    if let secondId = secondId, let secondIdRange = secondIdRange, let firstIdRange = firstIdRange {
                        statements.append(.edge(
                            from: firstId,
                            to: secondId,
                            sourceRange: range,
                            fromRange: firstIdRange,
                            toRange: secondIdRange
                        ))
                    } else {
                        // Malformed edge — treat as node definition
                        statements.append(.nodeDefinition(id: firstId, sourceRange: range))
                    }
                    i = stmtEnd
                    continue
                }
            }

            // Not an edge — it's a node definition (possibly with attributes)
            let stmtEnd = findStatementEnd(in: nsText, from: stmtStart, length: length)
            let range = NSRange(location: stmtStart, length: stmtEnd - stmtStart)
            statements.append(.nodeDefinition(id: firstId, sourceRange: range))
            i = stmtEnd
        }

        return DotGraph(statements: statements)
    }

    // MARK: - Character Constants

    private static let space: unichar = 0x20
    private static let tab: unichar = 0x09
    private static let newline: unichar = 0x0A
    private static let carriageReturn: unichar = 0x0D
    private static let semicolon: unichar = 0x3B
    private static let openBrace: unichar = 0x7B
    private static let closeBrace: unichar = 0x7D
    private static let openBracket: unichar = 0x5B
    private static let closeBracket: unichar = 0x5D
    private static let doubleQuote: unichar = 0x22
    private static let backslash: unichar = 0x5C
    private static let forwardSlash: unichar = 0x2F
    private static let asterisk: unichar = 0x2A
    private static let dash: unichar = 0x2D
    private static let greaterThan: unichar = 0x3E

    // MARK: - Scanning Helpers

    private static func isIdentChar(_ ch: unichar) -> Bool {
        return (ch >= 0x61 && ch <= 0x7A) || (ch >= 0x41 && ch <= 0x5A) ||
               (ch >= 0x30 && ch <= 0x39) || ch == 0x5F
    }

    private static func skipWhitespaceAndSemicolons(in text: NSString, from start: Int, length: Int) -> Int {
        var i = start
        while i < length {
            let ch = text.character(at: i)
            if ch == space || ch == tab || ch == newline || ch == carriageReturn || ch == semicolon {
                i += 1
            } else {
                break
            }
        }
        return i
    }

    private static func skipWhitespaceOnly(in text: NSString, from start: Int, length: Int) -> Int {
        var i = start
        while i < length {
            let ch = text.character(at: i)
            if ch == space || ch == tab || ch == newline || ch == carriageReturn {
                i += 1
            } else {
                break
            }
        }
        return i
    }

    private static func skipToEndOfLine(in text: NSString, from start: Int, length: Int) -> Int {
        var i = start
        while i < length && text.character(at: i) != newline {
            i += 1
        }
        if i < length { i += 1 }  // skip the newline itself
        return i
    }

    private static func skipBlockComment(in text: NSString, from start: Int, length: Int) -> Int {
        var i = start + 2  // skip past /*
        while i + 1 < length {
            if text.character(at: i) == asterisk && text.character(at: i + 1) == forwardSlash {
                return i + 2
            }
            i += 1
        }
        return length  // unterminated comment
    }

    /// Extract an identifier starting at the given position. Handles both bare
    /// identifiers (alphanumeric + underscore) and double-quoted identifiers.
    /// Returns (id, idRange, positionAfterIdentifier) or (nil, nil, start) if no identifier found.
    private static func extractIdentifier(in text: NSString, from start: Int, length: Int) -> (String?, NSRange?, Int) {
        guard start < length else { return (nil, nil, start) }

        let ch = text.character(at: start)

        if ch == doubleQuote {
            // Quoted identifier
            var i = start + 1
            while i < length {
                let c = text.character(at: i)
                if c == backslash && i + 1 < length {
                    i += 2  // skip escaped character
                    continue
                }
                if c == doubleQuote {
                    // The ID is the content without quotes for matching purposes
                    let idRange = NSRange(location: start, length: i + 1 - start)
                    let content = text.substring(with: NSRange(location: start + 1, length: i - start - 1))
                    return (content, idRange, i + 1)
                }
                i += 1
            }
            // Unterminated quote — treat as not an identifier
            return (nil, nil, start)
        }

        if isIdentChar(ch) {
            var i = start
            while i < length && isIdentChar(text.character(at: i)) {
                i += 1
            }
            let idRange = NSRange(location: start, length: i - start)
            let id = text.substring(with: idRange)
            return (id, idRange, i)
        }

        return (nil, nil, start)
    }

    /// Find the end of a statement starting at the given position.
    /// Tracks bracket depth, string literals, and comments to find the boundary.
    private static func findStatementEnd(in text: NSString, from start: Int, length: Int) -> Int {
        var i = start
        var bracketDepth = 0
        var inString = false

        while i < length {
            let ch = text.character(at: i)

            // Handle string literals
            if ch == doubleQuote && !inString {
                inString = true
                i += 1
                continue
            }
            if inString {
                if ch == backslash && i + 1 < length {
                    i += 2  // skip escape
                    continue
                }
                if ch == doubleQuote {
                    inString = false
                }
                i += 1
                continue
            }

            // Handle comments inside statements
            if ch == forwardSlash && i + 1 < length {
                let next = text.character(at: i + 1)
                if next == forwardSlash {
                    // Line comment ends the statement visually, but the statement
                    // range should stop before the comment
                    if bracketDepth == 0 {
                        return i
                    }
                    i = skipToEndOfLine(in: text, from: i, length: length)
                    continue
                }
                if next == asterisk {
                    i = skipBlockComment(in: text, from: i, length: length)
                    continue
                }
            }

            // Track bracket depth
            if ch == openBracket {
                bracketDepth += 1
                i += 1
                continue
            }
            if ch == closeBracket {
                bracketDepth -= 1
                if bracketDepth <= 0 {
                    return i + 1  // include the closing bracket
                }
                i += 1
                continue
            }

            // Statement boundaries (only at bracket depth 0)
            if bracketDepth == 0 {
                if ch == newline || ch == semicolon || ch == openBrace || ch == closeBrace {
                    return i
                }
            }

            i += 1
        }
        return length
    }

    /// Skip forward to the next statement boundary (newline, semicolon, or brace)
    /// while respecting strings, comments, and bracket nesting.
    private static func skipToStatementBoundary(in text: NSString, from start: Int, length: Int) -> Int {
        return findStatementEnd(in: text, from: start, length: length)
    }
}
