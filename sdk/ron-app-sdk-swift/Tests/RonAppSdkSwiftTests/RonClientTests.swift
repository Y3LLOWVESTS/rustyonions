import XCTest
@testable import RonAppSdkSwift

final class RonClientTests: XCTestCase {
    func testClientStoresConfig() throws {
        let url = URL(string: "https://example.com")!
        let config = RonConfig(baseUrl: url)
        let client = try RonClient(config: config)
        XCTAssertEqual(client.config.baseUrl, url)
    }

    func testInitDisallowsPlainHttpByDefault() {
        let url = URL(string: "http://example.com")!
        let config = RonConfig(baseUrl: url)

        XCTAssertThrowsError(try RonClient(config: config))
    }
}
