// ABOUTME: Main app entry point for the Dot Viewer macOS application.
// ABOUTME: Uses DocumentGroup to support multi-file tabbed editing of .dot files.

import SwiftUI

@main
struct DotViewerApp: App {
    var body: some Scene {
        DocumentGroup(newDocument: { DotDocument() }) { file in
            ContentView(document: file.document)
        }
    }
}
