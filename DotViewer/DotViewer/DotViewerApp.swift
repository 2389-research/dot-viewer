// ABOUTME: Main app entry point for the Dot Viewer macOS application.
// ABOUTME: Uses DocumentGroup to support multi-file tabbed editing of .dot files.

import SwiftUI
import AppKit

@main
struct DotViewerApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        DocumentGroup(newDocument: { DotDocument() }) { file in
            ContentView(document: file.document)
        }
    }
}

class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        // Force all document windows to open as tabs
        NSWindow.allowsAutomaticWindowTabbing = true
    }

    func applicationDidUpdate(_ notification: Notification) {
        // Merge any new windows into tabs of the first window.
        // Filter to only document windows by requiring a toolbar (which
        // DocumentGroup windows have). Internal AppKit/WebKit helper windows
        // lack toolbars and crash on addTabbedWindow due to unsupported
        // titlebarAccessoryViewControllers.
        let windows = NSApp.windows.filter {
            $0.isVisible &&
            $0.tabbingMode != .disallowed &&
            $0.toolbar != nil
        }
        guard windows.count > 1, let primary = windows.first else { return }

        for window in windows.dropFirst() {
            if window.tabbedWindows == nil || window.tabbedWindows?.contains(primary) == false {
                primary.addTabbedWindow(window, ordered: .above)
            }
        }
    }
}
