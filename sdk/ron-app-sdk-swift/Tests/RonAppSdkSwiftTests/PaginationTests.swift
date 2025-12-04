import XCTest
@testable import RonAppSdkSwift

final class PaginationTests: XCTestCase {

    func testPageDecodes() throws {
        let json = #"{"items":[1,2,3],"nextPageToken":null}"#
        let data = Data(json.utf8)

        let page = try JSONDecoder().decode(Page<Int>.self, from: data)

        XCTAssertEqual(page.items, [1, 2, 3])
        XCTAssertNil(page.nextPageToken)
    }

    func testPageDecodesWithNextPageToken() throws {
        let json = #"{"items":[42],"nextPageToken":"next-token"}"#
        let data = Data(json.utf8)

        let page = try JSONDecoder().decode(Page<Int>.self, from: data)

        XCTAssertEqual(page.items, [42])
        XCTAssertEqual(page.nextPageToken, "next-token")
    }
}
