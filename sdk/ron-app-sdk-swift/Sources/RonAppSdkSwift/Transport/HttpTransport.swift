//
// HttpTransport.swift
// ron-app-sdk-swift
//
// RO:WHAT  — Small wrapper around URLSession for RON app SDK.
// RO:WHY   — Centralize timeouts and logging; hide URLSession from public API.
// RO:INVARIANTS —
//   - Uses an ephemeral URLSession (no cookies, no disk caching).
//   - Uses RonConfig request/overall timeouts.
//   - Returns (Data, HTTPURLResponse) or throws a local transport error.
//

import Foundation

final class HttpTransport {
    private let session: URLSession
    private let logger: RonLogger
    private let config: RonConfig

    init(config: RonConfig, logger: RonLogger) {
        self.config = config
        self.logger = logger
        self.session = UrlSessionConfig.makeSession(config: config)
    }

    func send(_ request: URLRequest) async throws -> (Data, HTTPURLResponse) {
        logger.debug("HttpTransport.send \(request.httpMethod ?? "GET") \(request.url?.absoluteString ?? "<nil>")")

        let (data, response) = try await session.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw RonProblem.localTransportError(
                message: "Non-HTTP response from URLSession for \(request.url?.absoluteString ?? "<nil>")"
            )
        }

        logger.debug("HttpTransport.recv status=\(httpResponse.statusCode) bytes=\(data.count)")

        return (data, httpResponse)
    }
}
