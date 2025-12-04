//
// RequestBuilder.swift
// ron-app-sdk-swift
//
// RO:WHAT  — Turn an internal AppRequest into a Foundation.URLRequest.
// RO:WHY   — Keep URL construction, headers, and app-prefix/query handling encapsulated.
// RO:INVARIANTS —
//   - Always resolves paths under the `/app` prefix unless already present.
//   - Applies AppRequest.queryItems to the URL.
//   - Adds an Accept header if caller didn’t specify one.
//

import Foundation

enum RequestBuilder {
    enum Error: Swift.Error {
        case invalidBaseUrl(String)
        case invalidPath(String)
    }

    /// Build a URLRequest from our internal AppRequest + RonConfig.
    static func build(_ app: AppRequest, config: RonConfig) throws -> URLRequest {
        guard var components = URLComponents(
            url: config.baseUrl,
            resolvingAgainstBaseURL: false
        ) else {
            throw Error.invalidBaseUrl("Unable to resolve base URL: \(config.baseUrl.absoluteString)")
        }

        // Normalize path under /app prefix unless already provided.
        let pathUnderApp: String
        if app.path.hasPrefix("/app/") {
            pathUnderApp = app.path
        } else if app.path.hasPrefix("/") {
            pathUnderApp = "/app" + app.path
        } else {
            pathUnderApp = "/app/" + app.path
        }

        components.path = components.path.appending(pathUnderApp)

        // Attach query params if present.
        if !app.queryItems.isEmpty {
            components.queryItems = app.queryItems
        }

        guard let url = components.url else {
            throw Error.invalidPath(
                "Unable to construct URL from base + path: \(components) + \(pathUnderApp)"
            )
        }

        var request = URLRequest(url: url)
        request.httpMethod = app.method
        request.httpBody = app.body

        // Merge headers; allow caller to override defaults later.
        var headers = app.headers

        if headers["Accept"] == nil {
            headers["Accept"] = "application/json"
        }

        for (key, value) in headers {
            request.setValue(value, forHTTPHeaderField: key)
        }

        return request
    }
}
