// ABOUTME: Main app entry point for the Dot Viewer macOS application.
// ABOUTME: Uses DocumentGroup to support multi-file tabbed editing of .dot files.

import SwiftUI
import AppKit

@main
struct DotViewerApp: App {
    var body: some Scene {
        DocumentGroup(newDocument: { DotDocument() }) { file in
            ContentView(document: file.document)
                .onAppear {
                    // Force new documents to open as tabs in the existing window
                    if let window = NSApp.keyWindow ?? NSApp.windows.first {
                        window.tabbingMode = .preferred
                    }
                }
        }
    }
}
