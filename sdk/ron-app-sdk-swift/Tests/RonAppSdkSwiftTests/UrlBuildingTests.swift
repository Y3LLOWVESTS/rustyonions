import XCTest
@testable import RonAppSdkSwift

final class UrlBuildingTests: XCTestCase {
    func testBuildsAppPathUnderAppPrefix() throws {
        let url = URL(string: "https://example.com")!
        let config = RonConfig(baseUrl: url)

        let appReq = AppRequest(
            method: "GET",
            path: "/hello",
            queryItems: [URLQueryItem(name: "foo", value: "bar")],
            headers: [:],
            body: nil
        )

        let urlRequest = try RequestBuilder.build(appReq, config: config)

        XCTAssertEqual(
            urlRequest.url?.absoluteString,
            "https://example.com/app/hello?foo=bar"
        )
    }

    func testDoesNotDoublePrefixAppPath() throws {
        let url = URL(string: "https://example.com")!
        let config = RonConfig(baseUrl: url)

        let appReq = AppRequest(
            method: "GET",
            path: "/app/hello",
            queryItems: [],
            headers: [:],
            body: nil
        )

        let urlRequest = try RequestBuilder.build(appReq, config: config)

        XCTAssertEqual(urlRequest.url?.path, "/app/hello")
    }
}
