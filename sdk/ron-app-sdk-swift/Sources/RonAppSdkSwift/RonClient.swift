//
// RonClient.swift
// ron-app-sdk-swift
//
// RO:WHAT  — High-level Swift client for calling RON-CORE app plane.
// RO:WHY   — Give iOS/macOS devs a tiny, async/await-friendly wrapper around:
//            - config (RonConfig)
//            - transport (HttpTransport)
//            - request/response codecs (AppRequest/AppResponse/RonProblem).
// RO:INTERACTS —
//   - RonConfig        (base URL, timeouts, TLS mode)
//   - HttpTransport    (URLSession wrapper)
//   - RequestBuilder   (AppRequest -> URLRequest)
//   - ResponseParser   (Data + HTTPURLResponse -> AppResponse<T>)
// RO:INVARIANTS —
//   - Public call surface never throws; failures fold into AppResponse<RonProblem>.
//   - Low-level types (AppRequest) stay internal.
//   - HTTPS-only by default; plain HTTP requires explicit opt-in (allowInsecureHttp).
//

import Foundation

public final class RonClient {
    public let config: RonConfig

    private let transport: HttpTransport
    private let logger: RonLogger

    // MARK: - Lifecycle

    /// Primary initializer. Validates config invariants (e.g. HTTPS vs HTTP).
    ///
    /// May throw SdkConfigError when configuration is obviously unsafe or invalid.
    public init(config: RonConfig) throws {
        self.config = config
        self.logger = RonLogger()

        // Enforce HTTPS-by-default invariant.
        let scheme = config.baseUrl.scheme?.lowercased()

        if scheme == "http", !config.allowInsecureHttp {
            throw SdkConfigError(
                "Insecure HTTP base URL \(config.baseUrl.absoluteString) is not allowed; " +
                "set allowInsecureHttp = true for local dev only."
            )
        }

        self.transport = HttpTransport(config: config, logger: logger)
    }

    /// Convenience initializer that just takes a base URL.
    ///
    /// Uses default timeouts and disallows plain HTTP unless explicitly opted in later.
    public convenience init(baseUrl: URL) throws {
        try self.init(config: RonConfig(baseUrl: baseUrl))
    }

    // MARK: - Public high-level API

    /// Simple GET expecting a JSON body shaped like `AppResponse<T>`.
    ///
    /// Example:
    ///   let res: AppResponse<MyDto> = await client.get("/v1/hello")
    public func get<T: Decodable>(_ path: String) async -> AppResponse<T> {
        await getJson(path)
    }

    /// Explicit JSON GET helper (alias of `get` with a more descriptive name).
    ///
    /// Example:
    ///   let res: AppResponse<MyDto> = await client.getJson("/v1/hello")
    public func getJson<T: Decodable>(_ path: String) async -> AppResponse<T> {
        let req = AppRequest(
            method: "GET",
            path: path,
            queryItems: [],
            headers: ["Accept": "application/json"],
            body: nil
        )
        return await execute(appRequest: req)
    }

    /// POST JSON helper. Encodes `body` as JSON and expects `AppResponse<T>` back.
    ///
    /// Example:
    ///   struct HelloReq: Encodable { let name: String }
    ///   let res: AppResponse<MyDto> = await client.postJson("/v1/hello", body: HelloReq(name: "Stevan"))
    public func postJson<T: Decodable, Body: Encodable>(
        _ path: String,
        body: Body,
        headers: [String: String] = [:]
    ) async -> AppResponse<T> {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase

        do {
            let data = try encoder.encode(body)

            var mergedHeaders = headers
            mergedHeaders["Content-Type"] = "application/json"
            mergedHeaders["Accept"] = "application/json"

            let req = AppRequest(
                method: "POST",
                path: path,
                queryItems: [],
                headers: mergedHeaders,
                body: data
            )
            return await execute(appRequest: req)
        } catch {
            let problem = RonProblem.localEncodingError(
                message: "Failed to encode request body: \(error)"
            )
            return AppResponse(ok: false, value: nil, problem: problem)
        }
    }

    // MARK: - Internal low-level primitive

    /// Internal primitive: execute a fully-constructed AppRequest.
    ///
    /// Never throws; any local error (URL build, transport, parsing) becomes
    /// an AppResponse<T> with `ok == false` and a `RonProblem`.
    func execute<T: Decodable>(appRequest: AppRequest) async -> AppResponse<T> {
        do {
            let urlRequest = try RequestBuilder.build(appRequest, config: config)
            let (data, httpResponse) = try await transport.send(urlRequest)
            return ResponseParser.parse(data: data, response: httpResponse)
        } catch {
            let problem = RonProblem.localTransportOrBuildError(underlying: error)
            return AppResponse(ok: false, value: nil, problem: problem)
        }
    }
}
