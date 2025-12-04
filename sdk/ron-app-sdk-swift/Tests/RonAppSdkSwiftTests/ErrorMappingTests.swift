import XCTest
@testable import RonAppSdkSwift

final class ErrorMappingTests: XCTestCase {

    /// Non-2xx + bare `RonProblem` JSON should decode into `AppResponse<T>`
    /// with `ok == false`, `value == nil`, and a populated `RonProblem`.
    func testDecodesBareRonProblemOnNon2xx() throws {
        let json = """
        {
          "code": "upstream_unavailable",
          "message": "upstream says nope",
          "kind": "remote",
          "correlationId": "abc-123",
          "details": {
            "foo": "bar"
          }
        }
        """

        let data = Data(json.utf8)
        let response = HTTPURLResponse(
            url: URL(string: "https://example.com")!,
            statusCode: 503,
            httpVersion: nil,
            headerFields: [
                "Content-Type": "application/json"
            ]
        )!

        // Explicitly pin the generic so the compiler knows what T is.
        let appResponse: AppResponse<String> = ResponseParser.parse(
            data: data,
            response: response
        )

        XCTAssertFalse(appResponse.ok)
        XCTAssertNil(appResponse.value)

        let problem = try XCTUnwrap(appResponse.problem)
        XCTAssertEqual(problem.code, "upstream_unavailable")
        XCTAssertEqual(problem.message, "upstream says nope")
        XCTAssertEqual(problem.kind, "remote")
        XCTAssertEqual(problem.correlationId, "abc-123")
        XCTAssertEqual(problem.details?["foo"], CodableValue.string("bar"))
    }

    /// Non-2xx + non-JSON body should fall back to `RonProblem.localHttpStatus`.
    ///
    /// This is the Swift SDK’s local mapping:
    ///   - code:  "swift_sdk_http_status"
    ///   - kind:  "local"
    ///   - message: "Non-success HTTP status: <status>"
    ///   - details["httpStatus"] = .int(<status>)
    ///   - details["bodyPreview"] = truncated body (optional)
    func testLocalHttpStatusFallbackWhenBodyIsNotJson() throws {
        let body = "<html>503 Service Unavailable</html>"
        let data = Data(body.utf8)

        let response = HTTPURLResponse(
            url: URL(string: "https://example.com")!,
            statusCode: 503,
            httpVersion: nil,
            headerFields: [
                "Content-Type": "text/html"
            ]
        )!

        let appResponse: AppResponse<String> = ResponseParser.parse(
            data: data,
            response: response
        )

        XCTAssertFalse(appResponse.ok)
        XCTAssertNil(appResponse.value)

        let problem = try XCTUnwrap(appResponse.problem)
        XCTAssertEqual(problem.code, "swift_sdk_http_status")
        XCTAssertEqual(problem.message, "Non-success HTTP status: 503")
        XCTAssertEqual(problem.kind, "local")
        XCTAssertEqual(problem.details?["httpStatus"], CodableValue.int(503))

        // If we captured a preview, it should be a non-empty string.
        if let preview = problem.details?["bodyPreview"] {
            switch preview {
            case let .string(previewStr):
                XCTAssertFalse(previewStr.isEmpty)
            default:
                XCTFail("bodyPreview should be a string if present")
            }
        }
    }
}
