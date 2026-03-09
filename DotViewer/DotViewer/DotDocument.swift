// ABOUTME: Document model for .dot files using ReferenceFileDocument.
// ABOUTME: Handles file reading, writing, and change tracking for Graphviz DOT files.

import SwiftUI
import UniformTypeIdentifiers

extension UTType {
    static let dotFile = UTType(exportedAs: "com.2389.dot-viewer.dot",
                                conformingTo: .plainText)
    static let gvFile = UTType(exportedAs: "com.2389.dot-viewer.gv",
                                conformingTo: .plainText)
}

final class DotDocument: ReferenceFileDocument {
    typealias Snapshot = String

    @Published var text: String

    static var readableContentTypes: [UTType] { [.dotFile, .gvFile, .plainText] }
    static var writableContentTypes: [UTType] { [.dotFile, .gvFile, .plainText] }

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
