// ABOUTME: NSTextView wrapper providing a code editor for DOT files.
// ABOUTME: Supports basic syntax highlighting for DOT keywords, strings, and comments.

import SwiftUI
import AppKit

struct EditorView: NSViewRepresentable {
    @Binding var text: String

    func makeNSView(context: Context) -> NSScrollView {
        let scrollView = NSTextView.scrollableTextView()
        let textView = scrollView.documentView as! NSTextView

        // isRichText must be set to false before setting delegate and text
        textView.isRichText = false

        textView.font = NSFont.monospacedSystemFont(ofSize: 13, weight: .regular)
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticTextReplacementEnabled = false
        textView.allowsUndo = true
        textView.usesFindPanel = true

        textView.delegate = context.coordinator
        textView.string = text
        context.coordinator.applyHighlighting(to: textView)

        return scrollView
    }

    func updateNSView(_ scrollView: NSScrollView, context: Context) {
        let textView = scrollView.documentView as! NSTextView
        if textView.string != text {
            textView.string = text
            context.coordinator.applyHighlighting(to: textView)
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(text: $text)
    }

    class Coordinator: NSObject, NSTextViewDelegate {
        var text: Binding<String>

        private let keywords = [
            "digraph", "graph", "subgraph", "node", "edge", "strict"
        ]

        init(text: Binding<String>) {
            self.text = text
        }

        func textDidChange(_ notification: Notification) {
            guard let textView = notification.object as? NSTextView else { return }
            text.wrappedValue = textView.string
            applyHighlighting(to: textView)
        }

        func applyHighlighting(to textView: NSTextView) {
            let text = textView.string
            let fullRange = NSRange(location: 0, length: (text as NSString).length)
            let storage = textView.textStorage!

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

            // Strings (green): double-quoted strings
            if let regex = try? NSRegularExpression(pattern: "\"[^\"]*\"") {
                for match in regex.matches(in: text, range: fullRange) {
                    storage.addAttribute(.foregroundColor, value: NSColor.systemGreen, range: match.range)
                }
            }

            // Line comments (gray)
            if let regex = try? NSRegularExpression(pattern: "//[^\n]*") {
                for match in regex.matches(in: text, range: fullRange) {
                    storage.addAttribute(.foregroundColor, value: NSColor.systemGray, range: match.range)
                }
            }

            // Block comments (gray)
            if let regex = try? NSRegularExpression(pattern: "/\\*.*?\\*/", options: .dotMatchesLineSeparators) {
                for match in regex.matches(in: text, range: fullRange) {
                    storage.addAttribute(.foregroundColor, value: NSColor.systemGray, range: match.range)
                }
            }

            // Arrow operators (orange): -> and --
            if let regex = try? NSRegularExpression(pattern: "->|--") {
                for match in regex.matches(in: text, range: fullRange) {
                    storage.addAttribute(.foregroundColor, value: NSColor.systemOrange, range: match.range)
                }
            }
        }
    }
}
