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

    func testOffsetTranslationIsIdentityForPlainDot() {
        let doc = DotDocument()
        doc.loadDot(from: "digraph G { A -> B }")
        XCTAssertEqual(doc.dotOffsetForDippinOffset(5), 5)
        XCTAssertEqual(doc.dippinRangeForDotOffset(5)?.lowerBound, 5)
    }

    func testOffsetTranslationMapsThroughSourceMap() throws {
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

        let agentOffset = src.distance(from: src.startIndex,
                                       to: src.range(of: "agent A")!.lowerBound)
        let dotOffset = doc.dotOffsetForDippinOffset(agentOffset)
        XCTAssertNotNil(dotOffset)
        let entry = doc.sourceMap[0]
        XCTAssertTrue(Int(entry.dotStart)...Int(entry.dotEnd) ~= dotOffset!)

        let midDot = (Int(entry.dotStart) + Int(entry.dotEnd)) / 2
        let dipRange = doc.dippinRangeForDotOffset(midDot)
        XCTAssertNotNil(dipRange)
        XCTAssertTrue(dipRange!.contains(agentOffset))
    }

    func testOffsetTranslationBoundaryBehavior() throws {
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
        let entry = doc.sourceMap[0]

        // Inclusive lower bound
        XCTAssertEqual(doc.dotOffsetForDippinOffset(Int(entry.dipStart)), Int(entry.dotStart))
        // Exclusive upper bound returns nil
        XCTAssertNil(doc.dotOffsetForDippinOffset(Int(entry.dipEnd)))
        // Same for the reverse direction
        XCTAssertNotNil(doc.dippinRangeForDotOffset(Int(entry.dotStart)))
        XCTAssertNil(doc.dippinRangeForDotOffset(Int(entry.dotEnd)))
    }

    func testOffsetTranslationOutOfRangeReturnsNil() throws {
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
        XCTAssertNil(doc.dotOffsetForDippinOffset(999_999))
        XCTAssertNil(doc.dippinRangeForDotOffset(999_999))
    }

    func testOffsetTranslationFindsLaterEntry() throws {
        // Two agents → two source-map entries; verify the second is reachable.
        let src = """
        workflow F
          start: A
          exit: B
          agent A
            prompt: hi
            model: m
            provider: p
          agent B
            prompt: bye
            model: m
            provider: p
          edges
            A -> B
        """
        let doc = DotDocument()
        try doc.loadDippin(from: src)
        XCTAssertGreaterThan(doc.sourceMap.count, 1)
        let second = doc.sourceMap[1]
        let mid = (Int(second.dipStart) + Int(second.dipEnd)) / 2
        XCTAssertEqual(doc.dotOffsetForDippinOffset(mid), Int(second.dotStart))
    }
}
