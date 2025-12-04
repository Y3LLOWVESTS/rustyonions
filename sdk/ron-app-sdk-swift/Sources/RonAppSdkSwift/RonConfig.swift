/// RO:WHAT — SDK configuration: base URL, timeouts, auth/header providers, safety flags.
/// RO:WHY  — Central place to configure how the Swift SDK talks to RON-CORE.
/// RO:INTERACTS — Used by RonClient, HttpTransport, UrlSessionConfig, RequestBuilder.
/// RO:INVARIANTS —
///   - Immutable once constructed; safe to share across tasks.
///   - HTTPS-only by default; plain HTTP requires explicit opt-in.
///   - Timeouts are in milliseconds and always > 0 when used.
/// RO:METRICS/LOGS —
///   - Config decisions may influence timeout/latency metrics in transport.
/// RO:CONFIG —
///   - Reads `RON_SDK_GATEWAY_ADDR`, `RON_SDK_*_TIMEOUT_MS` via EnvConfig.
/// RO:SECURITY —
///   - No ambient caps: authToken/headerProvider are per-config, per-client.
/// RO:TEST_HOOKS —
///   - Covered by RonConfigTests and InteropTests.

import Foundation

/// Error type used when configuration is invalid or incomplete.
public struct SdkConfigError: Error, CustomStringConvertible {
    public let message: String

    public init(_ message: String) {
        self.message = message
    }

    public var description: String { message }
}

/// Immutable runtime configuration for RonClient.
public struct RonConfig {
    public typealias HeaderProvider = () async throws -> [String: String]

    public let baseUrl: URL
    public let overallTimeoutMs: Int
    public let connectTimeoutMs: Int
    public let readTimeoutMs: Int

    public let allowInsecureHttp: Bool
    public let debug: Bool

    public let authToken: String?
    public let headerProvider: HeaderProvider?

    public init(
        baseUrl: URL,
        overallTimeoutMs: Int = 10_000,
        connectTimeoutMs: Int = 3_000,
        readTimeoutMs: Int = 7_000,
        allowInsecureHttp: Bool = false,
        debug: Bool = false,
        authToken: String? = nil,
        headerProvider: HeaderProvider? = nil
    ) {
        self.baseUrl = baseUrl
        self.overallTimeoutMs = overallTimeoutMs
        self.connectTimeoutMs = connectTimeoutMs
        self.readTimeoutMs = readTimeoutMs
        self.allowInsecureHttp = allowInsecureHttp
        self.debug = debug
        self.authToken = authToken
        self.headerProvider = headerProvider
    }
}

/// Partial config used to override env-derived defaults.
public struct RonConfigOverrides {
    public var baseUrl: URL?
    public var overallTimeoutMs: Int?
    public var connectTimeoutMs: Int?
    public var readTimeoutMs: Int?

    public var allowInsecureHttp: Bool?
    public var debug: Bool?

    public var authToken: String?
    public var headerProvider: RonConfig.HeaderProvider?

    public init(
        baseUrl: URL? = nil,
        overallTimeoutMs: Int? = nil,
        connectTimeoutMs: Int? = nil,
        readTimeoutMs: Int? = nil,
        allowInsecureHttp: Bool? = nil,
        debug: Bool? = nil,
        authToken: String? = nil,
        headerProvider: RonConfig.HeaderProvider? = nil
    ) {
        self.baseUrl = baseUrl
        self.overallTimeoutMs = overallTimeoutMs
        self.connectTimeoutMs = connectTimeoutMs
        self.readTimeoutMs = readTimeoutMs
        self.allowInsecureHttp = allowInsecureHttp
        self.debug = debug
        self.authToken = authToken
        self.headerProvider = headerProvider
    }
}

public extension RonConfig {
    /// Build a RonConfig from canonical SDK env vars, with optional overrides.
    ///
    /// Precedence rules:
    ///   1. `overrides` values (if non-nil).
    ///   2. Env vars (`RON_SDK_GATEWAY_ADDR`, `RON_SDK_*_TIMEOUT_MS`).
    ///   3. Hard-coded defaults (10s overall, 3s connect, 7s read).
    ///
    /// Throws `SdkConfigError` if base URL is missing/invalid, or if insecure HTTP is used
    /// without `allowInsecureHttp` set to true.
    static func fromEnvironment(
        overrides: RonConfigOverrides = RonConfigOverrides()
    ) throws -> RonConfig {
        // Base URL.
        let baseUrl: URL? = {
            if let overrideUrl = overrides.baseUrl {
                return overrideUrl
            }

            if let rawUrl = EnvConfig.trimmedString("RON_SDK_GATEWAY_ADDR") {
                return URL(string: rawUrl)
            }

            return nil
        }()

        guard let resolvedBaseUrl = baseUrl else {
            throw SdkConfigError(
                "Missing base URL: set RON_SDK_GATEWAY_ADDR or provide overrides.baseUrl"
            )
        }

        // HTTPS vs HTTP check.
        let scheme = resolvedBaseUrl.scheme?.lowercased()
        let allowInsecure = overrides.allowInsecureHttp ?? false
        if scheme == "http", !allowInsecure {
            throw SdkConfigError(
                "Insecure HTTP base URL \(resolvedBaseUrl) is not allowed; " +
                "set overrides.allowInsecureHttp = true for local dev only."
            )
        }

        // Timeouts with env → overrides → default precedence.
        let overallTimeoutMs =
            overrides.overallTimeoutMs ??
            EnvConfig.int("RON_SDK_OVERALL_TIMEOUT_MS") ??
            10_000

        let connectTimeoutMs =
            overrides.connectTimeoutMs ??
            EnvConfig.int("RON_SDK_CONNECT_TIMEOUT_MS") ??
            3_000

        let readTimeoutMs =
            overrides.readTimeoutMs ??
            EnvConfig.int("RON_SDK_READ_TIMEOUT_MS") ??
            7_000

        // Other flags/hooks.
        let debug = overrides.debug ?? false
        let authToken = overrides.authToken
        let headerProvider = overrides.headerProvider

        return RonConfig(
            baseUrl: resolvedBaseUrl,
            overallTimeoutMs: overallTimeoutMs,
            connectTimeoutMs: connectTimeoutMs,
            readTimeoutMs: readTimeoutMs,
            allowInsecureHttp: allowInsecure,
            debug: debug,
            authToken: authToken,
            headerProvider: headerProvider
        )
    }
}
