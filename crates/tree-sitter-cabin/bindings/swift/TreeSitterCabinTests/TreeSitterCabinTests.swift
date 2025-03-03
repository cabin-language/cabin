import XCTest
import SwiftTreeSitter
import TreeSitterCabin

final class TreeSitterCabinTests: XCTestCase {
    func testCanLoadGrammar() throws {
        let parser = Parser()
        let language = Language(language: tree_sitter_cabin())
        XCTAssertNoThrow(try parser.setLanguage(language),
                         "Error loading Cabin grammar")
    }
}
