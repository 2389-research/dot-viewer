// ABOUTME: WKWebView wrapper that displays SVG output from Graphviz rendering.
// ABOUTME: Receives SVG strings and injects them into a minimal HTML shell.

import SwiftUI
import WebKit

struct PreviewView: NSViewRepresentable {
    let svgContent: String
    let errorMessage: String?

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        let webView = WKWebView(frame: .zero, configuration: config)
        webView.setValue(false, forKey: "drawsBackground")

        if let htmlURL = Bundle.main.url(forResource: "preview", withExtension: "html") {
            webView.loadFileURL(htmlURL, allowingReadAccessTo: htmlURL.deletingLastPathComponent())
        }

        context.coordinator.webView = webView
        webView.navigationDelegate = context.coordinator
        return webView
    }

    func updateNSView(_ webView: WKWebView, context: Context) {
        context.coordinator.updateContent(svg: svgContent, error: errorMessage)
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    class Coordinator: NSObject, WKNavigationDelegate {
        var webView: WKWebView?
        private var pendingSVG: String?
        private var pendingError: String?
        private var isLoaded = false

        func updateContent(svg: String, error: String?) {
            guard isLoaded, let webView else {
                pendingSVG = svg
                pendingError = error
                return
            }

            if let error {
                let escaped = error.replacingOccurrences(of: "\\", with: "\\\\")
                    .replacingOccurrences(of: "'", with: "\\'")
                    .replacingOccurrences(of: "\n", with: "\\n")
                webView.evaluateJavaScript("showError('\(escaped)')")
            } else if !svg.isEmpty {
                let escaped = svg.replacingOccurrences(of: "\\", with: "\\\\")
                    .replacingOccurrences(of: "'", with: "\\'")
                    .replacingOccurrences(of: "\n", with: "\\n")
                webView.evaluateJavaScript("updateSVG('\(escaped)')")
            }
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            isLoaded = true
            if let svg = pendingSVG {
                updateContent(svg: svg, error: pendingError)
                pendingSVG = nil
                pendingError = nil
            }
        }
    }
}
