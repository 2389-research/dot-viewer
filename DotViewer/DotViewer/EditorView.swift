// ABOUTME: NSTextView wrapper providing a code editor for DOT files.
// ABOUTME: Uses DotParser for cursor-to-node detection, with syntax highlighting, bracket matching, and auto-close.

import SwiftUI
import AppKit

/// Shared controller that allows ContentView to trigger editor navigation
/// without going through SwiftUI's binding/update cycle.
class EditorNavigator: ObservableObject {
    fileprivate weak var coordinator: EditorView.Coordinator?
    fileprivate weak var textView: NSTextView?

    func navigateToNode(_ nodeId: String) {
        guard let coordinator, let textView else { return }
        coordinator.navigateToNode(nodeId, in: textView)
    }
}

struct EditorView: NSViewRepresentable {
    @Binding var text: String
    @Binding var cursorNodeId: String?
    let navigator: EditorNavigator

    func makeNSView(context: Context) -> NSView {
        let container = NSView()

        let scrollView = NSTextView.scrollableTextView()
        let textView = scrollView.documentView as! NSTextView

        textView.isRichText = false
        textView.font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        textView.textColor = .textColor
        textView.backgroundColor = .textBackgroundColor
        textView.drawsBackground = true
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticTextReplacementEnabled = false
        textView.allowsUndo = true
        textView.usesFindPanel = true

        textView.string = text

        // Set delegate after text to avoid callbacks during setup
        textView.delegate = context.coordinator
        context.coordinator.dotGraph = DotParser.parse(text)
        context.coordinator.applyHighlighting(to: textView)

        // Wire up the navigator for direct calls from ContentView
        navigator.coordinator = context.coordinator
        navigator.textView = textView

        // Line number gutter — sits beside the scroll view, never touches it
        let gutter = LineNumberGutterView(textView: textView, scrollView: scrollView)

        container.addSubview(gutter)
        container.addSubview(scrollView)

        scrollView.translatesAutoresizingMaskIntoConstraints = false
        gutter.translatesAutoresizingMaskIntoConstraints = false

        let gutterWidthConstraint = gutter.widthAnchor.constraint(equalToConstant: gutter.requiredWidth)
        context.coordinator.gutterWidthConstraint = gutterWidthConstraint
        context.coordinator.gutterView = gutter
        context.coordinator.scrollView = scrollView

        NSLayoutConstraint.activate([
            gutter.leadingAnchor.constraint(equalTo: container.leadingAnchor),
            gutter.topAnchor.constraint(equalTo: container.topAnchor),
            gutter.bottomAnchor.constraint(equalTo: container.bottomAnchor),
            gutterWidthConstraint,

            scrollView.leadingAnchor.constraint(equalTo: gutter.trailingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: container.trailingAnchor),
            scrollView.topAnchor.constraint(equalTo: container.topAnchor),
            scrollView.bottomAnchor.constraint(equalTo: container.bottomAnchor),
        ])

        return container
    }

    func updateNSView(_ container: NSView, context: Context) {
        guard let scrollView = context.coordinator.scrollView,
              let textView = scrollView.documentView as? NSTextView else { return }
        if textView.string != text {
            textView.string = text
            context.coordinator.dotGraph = DotParser.parse(text)
            context.coordinator.applyHighlighting(to: textView)
        }
        // Navigation is handled directly via EditorNavigator, not through bindings.
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text, cursorNodeId: $cursorNodeId)
    }

    class Coordinator: NSObject, NSTextViewDelegate {
        var text: Binding<String>
        var cursorNodeId: Binding<String?>
        private var isNavigating = false
        private var isAutoClosing = false
        private var isProcessingSelectionChange = false
        fileprivate var dotGraph: DotGraph?
        var gutterWidthConstraint: NSLayoutConstraint?
        weak var gutterView: LineNumberGutterView?
        weak var scrollView: NSScrollView?

        private let keywords = [
            "digraph", "graph", "subgraph", "node", "edge", "strict"
        ]

        init(text: Binding<String>, cursorNodeId: Binding<String?>) {
            self.text = text
            self.cursorNodeId = cursorNodeId
        }

        func textView(_ textView: NSTextView, shouldChangeTextIn affectedCharRange: NSRange, replacementString: String?) -> Bool {
            guard !isAutoClosing else { return true }
            guard let replacement = replacementString, replacement.count == 1 else { return true }

            let closingBrackets: [String: String] = ["{": "}", "[": "]", "(": ")"]

            if let closing = closingBrackets[replacement] {
                isAutoClosing = true
                textView.insertText(replacement + closing, replacementRange: affectedCharRange)
                textView.setSelectedRange(NSRange(location: affectedCharRange.location + 1, length: 0))
                isAutoClosing = false
                return false
            }

            return true
        }

        func textDidChange(_ notification: Notification) {
            guard let textView = notification.object as? NSTextView else { return }
            text.wrappedValue = textView.string
            dotGraph = DotParser.parse(textView.string)
            applyHighlighting(to: textView)

            // Update gutter width if line count changed digit count
            if let gutter = gutterView, let constraint = gutterWidthConstraint {
                let newWidth = gutter.requiredWidth
                if abs(constraint.constant - newWidth) > 0.5 {
                    constraint.constant = newWidth
                }
            }
        }

        func textViewDidChangeSelection(_ notification: Notification) {
            guard !isProcessingSelectionChange else { return }
            guard let textView = notification.object as? NSTextView else { return }
            isProcessingSelectionChange = true
            defer { isProcessingSelectionChange = false }

            highlightMatchingBracket(in: textView)
            if !isNavigating {
                updateCursorNode(in: textView)
            }
        }

        // MARK: - Syntax Highlighting

        func applyHighlighting(to textView: NSTextView) {
            let text = textView.string
            let fullRange = NSRange(location: 0, length: (text as NSString).length)
            guard fullRange.length > 0 else { return }
            let storage = textView.textStorage!

            storage.beginEditing()

            // Reset all text to default color
            storage.addAttribute(.foregroundColor, value: NSColor.textColor, range: fullRange)

            // Keywords (purple)
            for keyword in keywords {
                let pattern = "\\b\(keyword)\\b"
                if let regex = try? NSRegularExpression(pattern: pattern) {
                    for match in regex.matches(in: text, range: fullRange) {
                        storage.addAttribute(.foregroundColor, value: NSColor.systemPurple, range: match.range)
                    }
                }
            }

            // Attribute names (blue): word before =
            if let regex = try? NSRegularExpression(pattern: "\\b(\\w+)\\s*=") {
                for match in regex.matches(in: text, range: fullRange) {
                    storage.addAttribute(.foregroundColor, value: NSColor.systemBlue, range: match.range(at: 1))
                }
            }

            // Strings (green): double-quoted strings — collect ranges for comment exclusion
            var stringRanges: [NSRange] = []
            if let regex = try? NSRegularExpression(pattern: "\"[^\"]*\"") {
                for match in regex.matches(in: text, range: fullRange) {
                    storage.addAttribute(.foregroundColor, value: NSColor.systemGreen, range: match.range)
                    stringRanges.append(match.range)
                }
            }

            // Line comments (gray) — skip matches inside quoted strings
            if let regex = try? NSRegularExpression(pattern: "//[^\n]*") {
                for match in regex.matches(in: text, range: fullRange) {
                    if !isInsideString(match.range, stringRanges: stringRanges) {
                        storage.addAttribute(.foregroundColor, value: NSColor.systemGray, range: match.range)
                    }
                }
            }

            // Block comments (gray) — skip matches inside quoted strings
            if let regex = try? NSRegularExpression(pattern: "/\\*.*?\\*/", options: .dotMatchesLineSeparators) {
                for match in regex.matches(in: text, range: fullRange) {
                    if !isInsideString(match.range, stringRanges: stringRanges) {
                        storage.addAttribute(.foregroundColor, value: NSColor.systemGray, range: match.range)
                    }
                }
            }

            // Arrow operators (orange): -> and --
            if let regex = try? NSRegularExpression(pattern: "->|--") {
                for match in regex.matches(in: text, range: fullRange) {
                    storage.addAttribute(.foregroundColor, value: NSColor.systemOrange, range: match.range)
                }
            }

            storage.endEditing()
        }

        /// Check if a range starts inside any quoted string
        private func isInsideString(_ range: NSRange, stringRanges: [NSRange]) -> Bool {
            for sr in stringRanges {
                if range.location >= sr.location && range.location < NSMaxRange(sr) {
                    return true
                }
            }
            return false
        }

        // MARK: - Bracket Matching

        private func highlightMatchingBracket(in textView: NSTextView) {
            let storage = textView.textStorage!
            let nsText = textView.string as NSString
            let fullRange = NSRange(location: 0, length: nsText.length)
            guard fullRange.length > 0 else { return }

            // Clear previous bracket highlights
            storage.beginEditing()
            storage.removeAttribute(.backgroundColor, range: fullRange)

            let cursorPos = textView.selectedRange().location

            let openBrackets: [unichar: unichar] = [
                unichar(0x7B): unichar(0x7D),  // { → }
                unichar(0x5B): unichar(0x5D),  // [ → ]
                unichar(0x28): unichar(0x29)   // ( → )
            ]
            let closeBrackets: [unichar: unichar] = [
                unichar(0x7D): unichar(0x7B),  // } → {
                unichar(0x5D): unichar(0x5B),  // ] → [
                unichar(0x29): unichar(0x28)   // ) → (
            ]

            var bracketPos: Int?
            var isOpen = false

            if cursorPos > 0 {
                let ch = nsText.character(at: cursorPos - 1)
                if openBrackets[ch] != nil {
                    bracketPos = cursorPos - 1
                    isOpen = true
                } else if closeBrackets[ch] != nil {
                    bracketPos = cursorPos - 1
                    isOpen = false
                }
            }
            if bracketPos == nil && cursorPos < nsText.length {
                let ch = nsText.character(at: cursorPos)
                if openBrackets[ch] != nil {
                    bracketPos = cursorPos
                    isOpen = true
                } else if closeBrackets[ch] != nil {
                    bracketPos = cursorPos
                    isOpen = false
                }
            }

            if let pos = bracketPos {
                let ch = nsText.character(at: pos)
                let highlightColor = NSColor.systemYellow.withAlphaComponent(0.3)

                if isOpen, let closeChar = openBrackets[ch] {
                    if let matchPos = findMatchingBracket(in: nsText, from: pos + 1, searchFor: closeChar, nestedBy: ch, forward: true) {
                        storage.addAttribute(.backgroundColor, value: highlightColor, range: NSRange(location: pos, length: 1))
                        storage.addAttribute(.backgroundColor, value: highlightColor, range: NSRange(location: matchPos, length: 1))
                    }
                } else if let openChar = closeBrackets[ch] {
                    if let matchPos = findMatchingBracket(in: nsText, from: pos - 1, searchFor: openChar, nestedBy: ch, forward: false) {
                        storage.addAttribute(.backgroundColor, value: highlightColor, range: NSRange(location: pos, length: 1))
                        storage.addAttribute(.backgroundColor, value: highlightColor, range: NSRange(location: matchPos, length: 1))
                    }
                }
            }

            storage.endEditing()
        }

        private func findMatchingBracket(in text: NSString, from start: Int, searchFor: unichar, nestedBy: unichar, forward: Bool) -> Int? {
            var depth = 1
            let step = forward ? 1 : -1
            var i = start

            while i >= 0 && i < text.length {
                let ch = text.character(at: i)
                if ch == nestedBy { depth += 1 }
                else if ch == searchFor { depth -= 1 }
                if depth == 0 { return i }
                i += step
            }
            return nil
        }

        // MARK: - Cursor Node Detection

        private func updateCursorNode(in textView: NSTextView) {
            let nodeId = findNodeAtCursor(in: textView)
            if nodeId != cursorNodeId.wrappedValue {
                cursorNodeId.wrappedValue = nodeId
            }
        }

        private func findNodeAtCursor(in textView: NSTextView) -> String? {
            guard let graph = dotGraph else { return nil }
            let pos = textView.selectedRange().location
            guard let stmt = graph.statementAt(offset: pos) else { return nil }
            return stmt.nodeIdAt(offset: pos)
        }

        // MARK: - Navigate to Node

        func navigateToNode(_ nodeId: String, in textView: NSTextView) {
            isNavigating = true
            defer { isNavigating = false }

            guard let graph = dotGraph,
                  let stmt = graph.definitionForNode(nodeId) else { return }

            let range = stmt.sourceRange
            guard range.location + range.length <= (textView.string as NSString).length else { return }
            textView.setSelectedRange(range)
            textView.scrollRangeToVisible(range)
        }
    }
}
