// ABOUTME: Tests DotDocument's handling of .dip source files.
// ABOUTME: Verifies dippin parsing on open and source-map population.

import XCTest
@testable import DotViewer

final class DotDocumentDippinTests: XCTestCase {
    func testOpenDippinFileParsesAndPopulatesSourceMap() throws {
        let src = """
        workflow F
          start: A
          exit: A
          agent A
            prompt: hi
            model: m
            provider: p
        """
        let doc = DotDocument()
        try doc.loadDippin(from: src)

        XCTAssertTrue(doc.isDippin)
        XCTAssertEqual(doc.text, src)
        XCTAssertTrue(doc.generatedDot.contains("digraph F {"))
        XCTAssertFalse(doc.sourceMap.isEmpty)
        XCTAssertNil(doc.parseError)
    }

    func testOpenInvalidDippinSetsParseError() {
        let doc = DotDocument()
        XCTAssertThrowsError(try doc.loadDippin(from: "workflow\n")) { _ in
            XCTAssertNotNil(doc.parseError)
        }
    }

    func testPlainDotDocumentIsNotDippin() throws {
        let doc = DotDocument()
        doc.loadDot(from: "digraph G { A -> B }")
        XCTAssertFalse(doc.isDippin)
        XCTAssertEqual(doc.generatedDot, doc.text)
        XCTAssertTrue(doc.sourceMap.isEmpty)
    }
}
