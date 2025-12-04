#!/usr/bin/env bash
set -euo pipefail

# Scaffold for ron-app-sdk-swift inside sdk/ron-app-sdk-swift

BASE_DIR="${1:-sdk/ron-app-sdk-swift}"

echo "Scaffolding ron-app-sdk-swift in: ${BASE_DIR}"

mkdir -p \
  "${BASE_DIR}" \
  "${BASE_DIR}/docs" \
  "${BASE_DIR}/Sources/RonAppSdkSwift" \
  "${BASE_DIR}/Sources/RonAppSdkSwift/Types" \
  "${BASE_DIR}/Sources/RonAppSdkSwift/Transport" \
  "${BASE_DIR}/Sources/RonAppSdkSwift/Logging" \
  "${BASE_DIR}/Sources/RonAppSdkSwift/Env" \
  "${BASE_DIR}/Sources/RonAppSdkSwift/Facets" \
  "${BASE_DIR}/Sources/RonAppSdkSwift/Internal" \
  "${BASE_DIR}/Tests/RonAppSdkSwiftTests" \
  "${BASE_DIR}/examples/hello-ron/Sources/HelloRon" \
  "${BASE_DIR}/examples/vapor-facet-demo/Sources/App" \
  "${BASE_DIR}/.github/workflows"

create_file_if_missing() {
  local path="$1"
  local content="$2"

  if [ -f "$path" ]; then
    echo "Skipping existing file: $path"
  else
    echo "Creating file: $path"
    # shellcheck disable=SC2016
    printf '%s\n' "$content" > "$path"
  fi
}

# Root files (light placeholders – you can overwrite with your real docs)

create_file_if_missing "${BASE_DIR}/README.md" "\
# ron-app-sdk-swift

This is the Swift SDK for talking to the RON-CORE App Plane via svc-gateway / omnigate.

See SDK_IDB.MD, SDK_SCHEMA_IDB.MD, and SDK_SECURITY.MD for design and invariants.
"

create_file_if_missing "${BASE_DIR}/SDK_IDB.MD" "\
<!-- SDK_IDB for ron-app-sdk-swift lives here. Replace this stub with the full IDB content. -->
"

create_file_if_missing "${BASE_DIR}/SDK_SCHEMA_IDB.MD" "\
<!-- SDK_SCHEMA_IDB for ron-app-sdk-swift lives here. Replace this stub with the full schema doc. -->
"

create_file_if_missing "${BASE_DIR}/SDK_SECURITY.MD" "\
<!-- SDK_SECURITY for ron-app-sdk-swift lives here. Replace this stub with the full security checklist. -->
"

create_file_if_missing "${BASE_DIR}/CHANGELOG.md" "\
# Changelog — ron-app-sdk-swift

All notable changes to this project will be documented in this file.
"

create_file_if_missing "${BASE_DIR}/LICENSE-MIT" "\
MIT License

<Fill with your standard MIT license text or link to workspace licenses.>
"

create_file_if_missing "${BASE_DIR}/LICENSE-APACHE" "\
Apache License 2.0

<Fill with your standard Apache-2.0 license text or link to workspace licenses.>
"

create_file_if_missing "${BASE_DIR}/.editorconfig" "\
root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
indent_style = space
indent_size = 4

[*.md]
indent_size = 2

[*.yml]
indent_size = 2

[*.swift]
indent_size = 4
"

create_file_if_missing "${BASE_DIR}/.gitignore" "\
.build/
.swiftpm/
.DS_Store
xcuserdata/
DerivedData/
*.xcodeproj/
.idea/
/*.log
"

# Package.swift — minimal SwiftPM manifest that compiles

create_file_if_missing "${BASE_DIR}/Package.swift" "\
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: \"ron-app-sdk-swift\",
    platforms: [
        .iOS(.v15),
        .macOS(.v12),
        .tvOS(.v15),
        .watchOS(.v8)
    ],
    products: [
        .library(
            name: \"RonAppSdkSwift\",
            targets: [\"RonAppSdkSwift\"]
        ),
    ],
    dependencies: [
        // Add SwiftLog or other dependencies here when needed.
        // .package(url: \"https://github.com/apple/swift-log.git\", from: \"1.0.0\"),
    ],
    targets: [
        .target(
            name: \"RonAppSdkSwift\",
            path: \"Sources/RonAppSdkSwift\"
        ),
        .testTarget(
            name: \"RonAppSdkSwiftTests\",
            dependencies: [\"RonAppSdkSwift\"],
            path: \"Tests/RonAppSdkSwiftTests\"
        ),
    ]
)
"

# docs/*.mmd placeholders

create_file_if_missing "${BASE_DIR}/docs/arch.mmd" "\
flowchart LR
  subgraph SwiftApp[Swift App / Service]
    A[Caller] --> B(RonClient)
  end

  B -->|HTTPS /app/*| C[svc-gateway / omnigate]
  C --> D[RON-CORE node]

  style B fill:#0b7285,stroke:#083344,color:#fff
"

create_file_if_missing "${BASE_DIR}/docs/sequence.mmd" "\
sequenceDiagram
  participant App as Swift App
  participant SDK as RonClient
  participant GW as svc-gateway
  participant Node as RON-CORE

  App->>SDK: request(/app/healthz)
  SDK->>GW: HTTPS /app/healthz
  GW->>Node: App Plane call
  Node-->>GW: Response/Problem
  GW-->>SDK: HTTP response
  SDK-->>App: AppResponse<T> / RonProblem
"

create_file_if_missing "${BASE_DIR}/docs/state.mmd" "\
stateDiagram-v2
  [*] --> Unconfigured
  Unconfigured --> Ready: RonClient.init(config)
  Ready --> InFlight: request()
  InFlight --> Ready: response_ok / problem
  InFlight --> Ready: timeout / transport_error
  Ready --> Shutdown: app_exit
  Shutdown --> [*]
"

# Sources/RonAppSdkSwift core files (comment-only stubs)

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/RonClient.swift" "\
/// RO:WHAT — Primary Swift client for the RON-CORE App Plane.
/// RO:WHY  — Provides async/await-friendly API over svc-gateway, with structured errors.
/// RO:INTERACTS — svc-gateway/omnigate (HTTP), RonConfig, HttpTransport, RonProblem/AppResponse.
/// RO:INVARIANTS —
///   - No global singletons; RonClient instances are explicit.
///   - Never logs secrets; always respects timeouts configured in RonConfig.

import Foundation

public final class RonClient {
    public let config: RonConfig

    public init(config: RonConfig) {
        self.config = config
    }

    // TODO: Implement request<T>(...) and helpers.
}
"

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/RonConfig.swift" "\
/// RO:WHAT — SDK configuration: base URL, timeouts, auth/header providers.
/// RO:WHY  — Central place to configure how the Swift SDK talks to RON-CORE.
/// RO:INVARIANTS — immutable once constructed; safe to share across tasks.

import Foundation

public struct RonConfig {
    public let baseUrl: URL
    public let overallTimeoutMs: Int
    public let connectTimeoutMs: Int
    public let readTimeoutMs: Int

    // Placeholder for header provider, logging hooks, PQ mode, etc.
    // public let headerProvider: (() async throws -> [String: String])?

    public init(
        baseUrl: URL,
        overallTimeoutMs: Int = 10_000,
        connectTimeoutMs: Int = 3_000,
        readTimeoutMs: Int = 7_000
    ) {
        self.baseUrl = baseUrl
        self.overallTimeoutMs = overallTimeoutMs
        self.connectTimeoutMs = connectTimeoutMs
        self.readTimeoutMs = readTimeoutMs
    }
}
"

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/AppRequest.swift" "\
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
"

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/AppResponse.swift" "\
/// Canonical AppResponse<T> wrapper for SDK calls.
/// Mirrors the shared schema: either a value or a RonProblem.

import Foundation

public struct AppResponse<T: Decodable>: Decodable {
    public let ok: Bool
    public let value: T?
    public let problem: RonProblem?

    public init(ok: Bool, value: T?, problem: RonProblem?) {
        self.ok = ok
        self.value = value
        self.problem = problem
    }
}
"

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/RonProblem.swift" "\
/// Canonical error envelope type for RON-CORE App Plane errors.
/// Matches the shared schema in SDK_SCHEMA_IDB.MD.

import Foundation

public struct RonProblem: Error, Codable {
    public let code: String
    public let message: String
    public let kind: String?
    public let correlationId: String?
    public let details: [String: CodableValue]?

    public init(
        code: String,
        message: String,
        kind: String? = nil,
        correlationId: String? = nil,
        details: [String: CodableValue]? = nil
    ) {
        self.code = code
        self.message = message
        self.kind = kind
        self.correlationId = correlationId
        self.details = details
    }
}
"

# Types

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Types/CodableValue.swift" "\
/// CodableValue — JSON \"any\" helper used for problem.details and other flexible payloads.

import Foundation

public enum CodableValue: Codable {
    case string(String)
    case int(Int)
    case double(Double)
    case bool(Bool)
    case array([CodableValue])
    case object([String: CodableValue])

    // TODO: Implement Codable conformance.
}
"

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Types/StringInt64.swift" "\
/// Wrapper for Int64 encoded/decoded as a JSON string (canonical i64 mapping).

import Foundation

public struct StringInt64: Codable {
    public let value: Int64

    public init(_ value: Int64) {
        self.value = value
    }
}
"

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Types/StringUInt64.swift" "\
/// Wrapper for UInt64 encoded/decoded as a JSON string (canonical u64 mapping).

import Foundation

public struct StringUInt64: Codable {
    public let value: UInt64

    public init(_ value: UInt64) {
        self.value = value
    }
}
"

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Types/Page.swift" "\
/// Generic Page<T> type for paginated responses.
/// Mirrors the canonical pagination envelope (items + next_page_token).

import Foundation

public struct Page<T: Decodable>: Decodable {
    public let items: [T]
    public let nextPageToken: String?
}
"

# Transport

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Transport/HttpTransport.swift" "\
/// HttpTransport — thin wrapper over URLSession used by RonClient.

import Foundation

struct HttpTransport {
    let session: URLSession

    // TODO: Implement send(request: AppRequest, config: RonConfig) async.
}
"

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Transport/UrlSessionConfig.swift" "\
/// UrlSessionConfig — helpers for building URLSessionConfiguration with timeouts etc.

import Foundation

enum UrlSessionConfig {
    static func makeSession(config: RonConfig) -> URLSession {
        let configuration = URLSessionConfiguration.default
        // TODO: Wire timeouts from RonConfig.
        return URLSession(configuration: configuration)
    }
}
"

# Logging

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Logging/RonLogger.swift" "\
/// RonLogger — small logging facade; can be wired to SwiftLog or custom logger.

import Foundation

public struct RonLogger {
    public init() {}

    public func debug(_ message: String) {
        // TODO: Integrate with actual logging backend.
        print(\"[ron-sdk-swift][DEBUG] \\(message)\")
    }
}
"

# Env

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Env/EnvConfig.swift" "\
/// EnvConfig — helpers for reading standard SDK env vars for ron-app-sdk-swift.

import Foundation

enum EnvConfig {
    static func string(_ key: String) -> String? {
        ProcessInfo.processInfo.environment[key]
    }
}
"

# Facets

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Facets/FacetClient.swift" "\
/// FacetClient — helper for addressing /facets/{id}{path} routes via RonClient.

import Foundation

public struct FacetClient {
    public let client: RonClient
    public let id: String

    public init(client: RonClient, id: String) {
        self.client = client
        self.id = id
    }
}
"

# Internal

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Internal/RequestBuilder.swift" "\
/// RequestBuilder — builds URLRequest from AppRequest + RonConfig.

import Foundation

enum RequestBuilder {
    // TODO: Implement URLRequest building logic.
}
"

create_file_if_missing "${BASE_DIR}/Sources/RonAppSdkSwift/Internal/ResponseParser.swift" "\
/// ResponseParser — parses HTTPURLResponse + Data into AppResponse<T> and RonProblem.

import Foundation

enum ResponseParser {
    // TODO: Implement parsing logic.
}
"

# Tests — minimal XCTest stubs

create_file_if_missing "${BASE_DIR}/Tests/RonAppSdkSwiftTests/RonConfigTests.swift" "\
import XCTest
@testable import RonAppSdkSwift

final class RonConfigTests: XCTestCase {
    func testInitWithDefaults() throws {
        let url = URL(string: \"https://example.com\")!
        let config = RonConfig(baseUrl: url)
        XCTAssertEqual(config.baseUrl, url)
    }
}
"

create_file_if_missing "${BASE_DIR}/Tests/RonAppSdkSwiftTests/RonClientTests.swift" "\
import XCTest
@testable import RonAppSdkSwift

final class RonClientTests: XCTestCase {
    func testClientStoresConfig() throws {
        let url = URL(string: \"https://example.com\")!
        let config = RonConfig(baseUrl: url)
        let client = RonClient(config: config)
        XCTAssertEqual(client.config.baseUrl, url)
    }
}
"

create_file_if_missing "${BASE_DIR}/Tests/RonAppSdkSwiftTests/ErrorMappingTests.swift" "\
import XCTest
@testable import RonAppSdkSwift

final class ErrorMappingTests: XCTestCase {
    func testProblemInit() throws {
        let problem = RonProblem(code: \"test\", message: \"Test problem\")
        XCTAssertEqual(problem.code, \"test\")
        XCTAssertEqual(problem.message, \"Test problem\")
    }
}
"

create_file_if_missing "${BASE_DIR}/Tests/RonAppSdkSwiftTests/PaginationTests.swift" "\
import XCTest
@testable import RonAppSdkSwift

final class PaginationTests: XCTestCase {
    func testPageDecodes() throws {
        let json = \"\"\"{\"items\":[1,2,3],\"nextPageToken\":null}\"\"\"
        let data = Data(json.utf8)
        let page = try JSONDecoder().decode(Page<Int>.self, from: data)
        XCTAssertEqual(page.items.count, 3)
    }
}
"

create_file_if_missing "${BASE_DIR}/Tests/RonAppSdkSwiftTests/UrlBuildingTests.swift" "\
import XCTest
@testable import RonAppSdkSwift

final class UrlBuildingTests: XCTestCase {
    func testDummy() throws {
        // TODO: Implement URL building tests when RequestBuilder is ready.
        XCTAssertTrue(true)
    }
}
"

create_file_if_missing "${BASE_DIR}/Tests/RonAppSdkSwiftTests/InteropTests.swift" "\
import XCTest
@testable import RonAppSdkSwift

final class InteropTests: XCTestCase {
    func testInteropPlaceholder() throws {
        // TODO: Wire this up to a real micronode/macronode in CI.
        XCTAssertTrue(true)
    }
}
"

# examples/hello-ron

create_file_if_missing "${BASE_DIR}/examples/hello-ron/Package.swift" "\
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: \"hello-ron\",
    platforms: [
        .iOS(.v15),
        .macOS(.v12)
    ],
    dependencies: [
        .package(path: \"../../\")
    ],
    targets: [
        .executableTarget(
            name: \"HelloRon\",
            dependencies: [
                .product(name: \"RonAppSdkSwift\", package: \"ron-app-sdk-swift\")
            ]
        )
    ]
)
"

create_file_if_missing "${BASE_DIR}/examples/hello-ron/Sources/HelloRon/main.swift" "\
import Foundation
import RonAppSdkSwift

@main
struct HelloRon {
    static func main() async {
        // TODO: Replace with real RonConfig.fromEnvironment once implemented.
        guard let url = URL(string: \"https://127.0.0.1:8090\") else {
            print(\"Invalid URL\")
            return
        }
        let config = RonConfig(baseUrl: url)
        let client = RonClient(config: config)
        print(\"Hello from ron-app-sdk-swift example — client base URL: \\(client.config.baseUrl)\")
    }
}
"

# examples/vapor-facet-demo (placeholder; implementation later)

create_file_if_missing "${BASE_DIR}/examples/vapor-facet-demo/Package.swift" "\
// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: \"vapor-facet-demo\",
    platforms: [
        .iOS(.v15),
        .macOS(.v12)
    ],
    dependencies: [
        // .package(url: \"https://github.com/vapor/vapor.git\", from: \"4.0.0\"),
        .package(path: \"../../\")
    ],
    targets: [
        .executableTarget(
            name: \"App\",
            dependencies: [
                .product(name: \"RonAppSdkSwift\", package: \"ron-app-sdk-swift\")
                // , .product(name: \"Vapor\", package: \"vapor\")
            ]
        )
    ]
)
"

create_file_if_missing "${BASE_DIR}/examples/vapor-facet-demo/Sources/App/main.swift" "\
import Foundation
// import Vapor
import RonAppSdkSwift

@main
struct VaporFacetDemo {
    static func main() async {
        // TODO: Wire into Vapor routes; for now just a placeholder.
        print(\"Vapor facet demo placeholder — integrate RonClient here.\")
    }
}
"

# GitHub Actions workflow stub

create_file_if_missing "${BASE_DIR}/.github/workflows/swift-ci.yml" "\
name: swift-ci

on:
  push:
    paths:
      - \"sdk/ron-app-sdk-swift/**\"
  pull_request:
    paths:
      - \"sdk/ron-app-sdk-swift/**\"

jobs:
  build-and-test:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: |
          cd sdk/ron-app-sdk-swift
          swift build
      - name: Test
        run: |
          cd sdk/ron-app-sdk-swift
          swift test
"

echo "Scaffold for ron-app-sdk-swift created (or updated) at: ${BASE_DIR}"
