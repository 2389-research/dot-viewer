// ABOUTME: Main content view with split-pane layout for editing and previewing DOT files.
// ABOUTME: Left pane is the text editor, right pane is the SVG preview.

import SwiftUI

struct ContentView: View {
    @ObservedObject var document: DotDocument

    var body: some View {
        HSplitView {
            TextEditor(text: $document.text)
                .font(.system(.body, design: .monospaced))
                .frame(minWidth: 300)

            Text("SVG Preview")
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .background(Color(nsColor: .controlBackgroundColor))
                .frame(minWidth: 300)
        }
        .frame(minWidth: 800, minHeight: 500)
        .onAppear {
            do {
                let svg = try renderDot(dotSource: "digraph { a -> b }", engine: .dot)
                print("Render succeeded: \(svg.prefix(100))...")
            } catch {
                print("Render failed: \(error)")
            }
        }
    }
}
