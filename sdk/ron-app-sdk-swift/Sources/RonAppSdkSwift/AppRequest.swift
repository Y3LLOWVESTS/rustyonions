/// Internal representation of a request issued via RonClient.
/// Encapsulates HTTP method, path, query params, headers, and optional body.

import Foundation

struct AppRequest {
    let method: String
    let path: String
    let queryItems: [URLQueryItem]
    let headers: [String: String]
    let body: Data?
}

