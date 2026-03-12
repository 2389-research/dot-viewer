// ABOUTME: Document model for .dot files using ReferenceFileDocument.
// ABOUTME: Handles file reading, writing, and change tracking for Graphviz DOT files.

import SwiftUI
import UniformTypeIdentifiers

extension UTType {
    // Custom exported types declared in Info.plist for .dot and .gv extensions.
    static let graphvizDot: UTType = UTType("com.2389.dot-viewer.dot") ?? .plainText
    static let graphvizGv: UTType = UTType("com.2389.dot-viewer.gv") ?? .plainText
    // macOS maps .dot to com.microsoft.word.dot (Word template), so we must
    // include that type to allow opening .dot files from the file picker.
    static let msWordDot: UTType = UTType("com.microsoft.word.dot") ?? .data
}

final class DotDocument: ReferenceFileDocument {
    typealias Snapshot = String

    @Published var text: String

    static var readableContentTypes: [UTType] { [.graphvizDot, .graphvizGv, .msWordDot, .plainText] }
    static var writableContentTypes: [UTType] { [.plainText] }

    init(text: String = "digraph {\n    a -> b\n    b -> c\n}") {
        self.text = text
    }

    init(configuration: ReadConfiguration) throws {
        guard let data = configuration.file.regularFileContents,
              let string = String(data: data, encoding: .utf8) else {
            throw CocoaError(.fileReadCorruptFile)
        }
        self.text = string
    }

    func snapshot(contentType: UTType) throws -> String {
        text
    }

    func fileWrapper(snapshot: String, configuration: WriteConfiguration) throws -> FileWrapper {
        let data = snapshot.data(using: .utf8)!
        return FileWrapper(regularFileWithContents: data)
    }
}
