import XCTest
@testable import RonAppSdkSwift

final class RonConfigTests: XCTestCase {
    func testInitWithDefaults() throws {
        let url = URL(string: "https://example.com")!
        let config = RonConfig(baseUrl: url)
        XCTAssertEqual(config.baseUrl, url)
    }
}

