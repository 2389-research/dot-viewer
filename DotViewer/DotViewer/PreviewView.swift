// ABOUTME: WKWebView wrapper that displays SVG output from Graphviz rendering.
// ABOUTME: Supports node highlighting and click-to-select for bidirectional editor linking.

import SwiftUI
import WebKit

struct PreviewView: NSViewRepresentable {
    let svgContent: String
    let errorMessage: String?
    var highlightedNodeId: String?
    var onNodeClicked: ((String) -> Void)?

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.userContentController.add(context.coordinator, name: "nodeClicked")

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
        context.coordinator.onNodeClicked = onNodeClicked
        context.coordinator.updateContent(svg: svgContent, error: errorMessage)
        context.coordinator.updateHighlight(nodeId: highlightedNodeId)
    }

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    class Coordinator: NSObject, WKNavigationDelegate, WKScriptMessageHandler {
        var webView: WKWebView?
        var onNodeClicked: ((String) -> Void)?
        private var pendingSVG: String?
        private var pendingError: String?
        private var isLoaded = false
        private var lastSVG: String?
        private var lastHighlightedNodeId: String?

        func updateContent(svg: String, error: String?) {
            guard isLoaded, let webView else {
                pendingSVG = svg
                pendingError = error
                return
            }

            // Only update if SVG actually changed
            guard svg != lastSVG else { return }
            lastSVG = svg

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

                // Re-apply highlight after SVG update
                if let nodeId = lastHighlightedNodeId {
                    let escapedNode = nodeId.replacingOccurrences(of: "\\", with: "\\\\")
                        .replacingOccurrences(of: "'", with: "\\'")
                    webView.evaluateJavaScript("highlightNode('\(escapedNode)')")
                }
            }
        }

        func updateHighlight(nodeId: String?) {
            guard isLoaded, let webView else {
                lastHighlightedNodeId = nodeId
                return
            }

            guard nodeId != lastHighlightedNodeId else { return }
            lastHighlightedNodeId = nodeId

            if let nodeId {
                let escaped = nodeId.replacingOccurrences(of: "\\", with: "\\\\")
                    .replacingOccurrences(of: "'", with: "\\'")
                webView.evaluateJavaScript("highlightNode('\(escaped)')")
            } else {
                webView.evaluateJavaScript("clearHighlights()")
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

        // MARK: - WKScriptMessageHandler

        func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
            if message.name == "nodeClicked", let nodeId = message.body as? String {
                // For edges like "a->b" or "a--b", extract the first node name.
                // Split on arrow operators, not individual hyphens, to preserve
                // node names containing hyphens (e.g. "my-node").
                let cleanId: String
                if let arrowRange = nodeId.range(of: "->") {
                    cleanId = String(nodeId[..<arrowRange.lowerBound]).trimmingCharacters(in: .whitespaces)
                } else if let dashRange = nodeId.range(of: "--") {
                    cleanId = String(nodeId[..<dashRange.lowerBound]).trimmingCharacters(in: .whitespaces)
                } else {
                    cleanId = nodeId.trimmingCharacters(in: .whitespaces)
                }
                DispatchQueue.main.async {
                    self.onNodeClicked?(cleanId)
                }
            }
        }
    }
}
