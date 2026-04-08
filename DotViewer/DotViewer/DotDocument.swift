// ABOUTME: Document model for .dot and .dip files using ReferenceFileDocument.
// ABOUTME: Handles file reading, writing, change tracking, and dippin parsing state.

import SwiftUI
import UniformTypeIdentifiers

extension UTType {
    // Custom exported types declared in Info.plist for .dot and .gv extensions.
    static let graphvizDot: UTType = UTType("com.2389.dot-viewer.dot") ?? .plainText
    static let graphvizGv: UTType = UTType("com.2389.dot-viewer.gv") ?? .plainText
    // macOS maps .dot to com.microsoft.word.dot (Word template), so we must
    // include that type to allow opening .dot files from the file picker.
    static let msWordDot: UTType = UTType("com.microsoft.word.dot") ?? .data
    // Dippin source files (.dip) — a higher-level DSL that compiles to DOT.
    static let dippin: UTType = UTType("com.2389.dot-viewer.dip") ?? .plainText
}

final class DotDocument: ReferenceFileDocument {
    typealias Snapshot = String

    @Published var text: String

    // Dippin-related state. For plain DOT documents, isDippin is false,
    // generatedDot mirrors text, and sourceMap is empty.
    @Published private var _isDippin: Bool = false
    @Published private var _generatedDot: String = ""
    @Published private var _sourceMap: [SourceMapEntry] = []
    @Published private var _parseError: String? = nil

    var isDippin: Bool { _isDippin }
    var generatedDot: String { _generatedDot }
    var sourceMap: [SourceMapEntry] { _sourceMap }
    var parseError: String? { _parseError }

    static var readableContentTypes: [UTType] { [.graphvizDot, .graphvizGv, .msWordDot, .dippin, .plainText] }
    static var writableContentTypes: [UTType] { [.plainText] }

    init(text: String = "digraph {\n    a -> b\n    b -> c\n}") {
        self.text = text
        // Default documents are plain DOT, so generatedDot mirrors the source.
        self._generatedDot = text
    }

    init(configuration: ReadConfiguration) throws {
        guard let data = configuration.file.regularFileContents,
              let source = String(data: data, encoding: .utf8) else {
            throw CocoaError(.fileReadCorruptFile)
        }
        // text must be initialized before invoking instance methods.
        self.text = source
        if configuration.contentType == .dippin {
            try self.loadDippin(from: source)
        } else {
            self.loadDot(from: source)
        }
    }

    func snapshot(contentType: UTType) throws -> String {
        text
    }

    func fileWrapper(snapshot: String, configuration: WriteConfiguration) throws -> FileWrapper {
        let data = snapshot.data(using: .utf8)!
        return FileWrapper(regularFileWithContents: data)
    }

    // Load a plain DOT source, resetting any dippin-specific state.
    func loadDot(from source: String) {
        self.text = source
        self._isDippin = false
        self._generatedDot = source
        self._sourceMap = []
        self._parseError = nil
    }

    // Load a dippin source, parse it to DOT, and populate the source map.
    // On failure, stores the error message on the document and rethrows so
    // the caller can surface the failure (e.g. alert the user).
    func loadDippin(from source: String) throws {
        self.text = source
        self._isDippin = true
        do {
            let result = try parseDippin(source: source)
            self._generatedDot = result.dotSource
            self._sourceMap = result.sourceMap
            self._parseError = nil
        } catch let error as DotError {
            let msg: String
            switch error {
            case .SyntaxError(let m, _, _):
                msg = m
            case .LayoutError(let m):
                msg = m
            case .RenderError(let m):
                msg = m
            }
            self._generatedDot = ""
            self._sourceMap = []
            self._parseError = msg
            throw error
        } catch {
            self._generatedDot = ""
            self._sourceMap = []
            self._parseError = "\(error)"
            throw error
        }
    }

    // Re-parse the current text as dippin (if this is a dippin document).
    // On error, the previous generatedDot is preserved so the preview does
    // not blank out during transient edits.
    func reparseDippinIfNeeded() {
        guard _isDippin else {
            _generatedDot = text
            return
        }
        do {
            let result = try parseDippin(source: text)
            self._generatedDot = result.dotSource
            self._sourceMap = result.sourceMap
            self._parseError = nil
        } catch let error as DotError {
            switch error {
            case .SyntaxError(let m, _, _):
                self._parseError = m
            case .LayoutError(let m):
                self._parseError = m
            case .RenderError(let m):
                self._parseError = m
            }
            // Keep previous generatedDot so preview doesn't blank on transient error.
        } catch {
            self._parseError = "\(error)"
        }
    }

    /// Translate a dippin-space offset to DOT space, collapsing to the start of
    /// the generated DOT construct (point, not range). Returns identity for
    /// non-dippin docs. Uses half-open `[dipStart, dipEnd)` matching, so an
    /// offset exactly at `dipEnd` returns nil. Assumes source-map entries are
    /// non-overlapping; first match wins.
    func dotOffsetForDippinOffset(_ dipOffset: Int) -> Int? {
        if !_isDippin { return dipOffset }
        for entry in _sourceMap {
            let dipStart = Int(entry.dipStart)
            let dipEnd = Int(entry.dipEnd)
            if dipOffset >= dipStart && dipOffset < dipEnd {
                return Int(entry.dotStart)
            }
        }
        return nil
    }

    /// Translate a DOT-space offset to the full dippin-space range of the
    /// originating construct (asymmetric with `dotOffsetForDippinOffset`, which
    /// collapses to a point). Returns `offset..<offset` for non-dippin docs.
    /// Uses half-open `[dotStart, dotEnd)` matching, so an offset exactly at
    /// `dotEnd` returns nil. Assumes non-overlapping entries; first match wins.
    func dippinRangeForDotOffset(_ dotOffset: Int) -> Range<Int>? {
        if !_isDippin { return dotOffset..<dotOffset }
        for entry in _sourceMap {
            let dotStart = Int(entry.dotStart)
            let dotEnd = Int(entry.dotEnd)
            if dotOffset >= dotStart && dotOffset < dotEnd {
                return Int(entry.dipStart)..<Int(entry.dipEnd)
            }
        }
        return nil
    }
}
