// ABOUTME: Main content view with split-pane layout for editing and previewing DOT files.
// ABOUTME: Left pane is the text editor, right pane is the live SVG preview.

import SwiftUI

struct ContentView: View {
    @ObservedObject var document: DotDocument
    @State private var svgOutput: String = ""
    @State private var errorMessage: String?
    @State private var selectedEngine: LayoutEngine = .dot
    @State private var liveMode: Bool = true
    @State private var renderTask: Task<Void, Never>?

    var body: some View {
        HSplitView {
            VStack(spacing: 0) {
                EditorView(text: $document.text)
                    .onChange(of: document.text) {
                        if liveMode {
                            scheduleRender()
                        }
                    }

                if let errorMessage {
                    HStack {
                        Image(systemName: "exclamationmark.triangle.fill")
                            .foregroundColor(.red)
                        Text(errorMessage)
                            .font(.system(.caption, design: .monospaced))
                            .foregroundColor(.red)
                            .lineLimit(2)
                        Spacer()
                    }
                    .padding(8)
                    .background(Color.red.opacity(0.1))
                }
            }
            .frame(minWidth: 300)

            PreviewView(svgContent: svgOutput, errorMessage: errorMessage)
                .frame(minWidth: 300)
        }
        .frame(minWidth: 800, minHeight: 500)
        .toolbar {
            ToolbarItem {
                Picker("Engine", selection: $selectedEngine) {
                    Text("dot").tag(LayoutEngine.dot)
                    Text("neato").tag(LayoutEngine.neato)
                    Text("fdp").tag(LayoutEngine.fdp)
                    Text("circo").tag(LayoutEngine.circo)
                    Text("twopi").tag(LayoutEngine.twopi)
                    Text("sfdp").tag(LayoutEngine.sfdp)
                }
                .frame(width: 100)
            }
            ToolbarItem {
                Toggle("Live", isOn: $liveMode)
                    .toggleStyle(.switch)
            }
            ToolbarItem {
                Button("Refresh") {
                    performRender()
                }
                .keyboardShortcut("r", modifiers: .command)
            }
        }
        .onChange(of: selectedEngine) {
            performRender()
        }
        .onAppear {
            performRender()
        }
    }

    private func scheduleRender() {
        renderTask?.cancel()
        renderTask = Task {
            try? await Task.sleep(nanoseconds: 300_000_000)
            if !Task.isCancelled {
                performRender()
            }
        }
    }

    private func performRender() {
        let source = document.text
        let engine = selectedEngine
        Task.detached {
            do {
                let svg = try renderDot(dotSource: source, engine: engine)
                await MainActor.run {
                    svgOutput = svg
                    errorMessage = nil
                }
            } catch {
                await MainActor.run {
                    errorMessage = "\(error)"
                }
            }
        }
    }
}
