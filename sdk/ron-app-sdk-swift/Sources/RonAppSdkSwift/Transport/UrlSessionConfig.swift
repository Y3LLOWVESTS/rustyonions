//
// UrlSessionConfig.swift
// ron-app-sdk-swift
//
// RO:WHAT  — Factory for URLSession configured for RON app SDK.
// RO:WHY   — Single place to tune timeouts, caching, and UA.
//

import Foundation

enum UrlSessionConfig {
    static func makeSession(config: RonConfig) -> URLSession {
        // Ephemeral configuration — no persistent cookies or caches.
        let cfg = URLSessionConfiguration.ephemeral

        // Timeouts from RonConfig (ms -> seconds).
        let overallSeconds = TimeInterval(config.overallTimeoutMs) / 1000.0
        cfg.timeoutIntervalForRequest = overallSeconds
        cfg.timeoutIntervalForResource = overallSeconds

        // No cookies or cache on disk; we keep this simple and predictable.
        cfg.httpCookieStorage = nil
        cfg.requestCachePolicy = .reloadIgnoringLocalAndRemoteCacheData
        cfg.urlCache = nil

        var headers: [AnyHashable: Any] = [:]
        headers["User-Agent"] = "ron-app-sdk-swift/0.1.0-dev"
        cfg.httpAdditionalHeaders = headers

        return URLSession(configuration: cfg)
    }
}
