//
// RonProblem.swift
// ron-app-sdk-swift
//
// RO:WHAT  — Problem detail object for RON-CORE + local SDK errors.
// RO:WHY   — Single shape for all error flows, remote and local.
//

import Foundation

public struct RonProblem: Codable, Error {
    public let code: String
    public let message: String
    public let kind: String
    public let correlationId: String?
    public let details: [String: CodableValue]?

    public init(
        code: String,
        message: String,
        kind: String,
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

// MARK: - Local SDK error helpers

public extension RonProblem {
    static func localTransportError(message: String) -> RonProblem {
        RonProblem(
            code: "swift_sdk_transport_error",
            message: message,
            kind: "local",
            correlationId: nil,
            details: nil
        )
    }

    static func localTransportOrBuildError(underlying: Error) -> RonProblem {
        RonProblem(
            code: "swift_sdk_transport_error",
            message: "Transport or request build failed: \(underlying)",
            kind: "local",
            correlationId: nil,
            details: [
                "underlying": .string(String(describing: underlying))
            ]
        )
    }

    static func localEncodingError(message: String) -> RonProblem {
        RonProblem(
            code: "swift_sdk_encoding_error",
            message: message,
            kind: "local",
            correlationId: nil,
            details: nil
        )
    }

    static func localDecodingError(status: Int?, message: String) -> RonProblem {
        var details: [String: CodableValue] = [
            "message": .string(message)
        ]
        if let status {
            details["httpStatus"] = .int(status)
        }

        return RonProblem(
            code: "swift_sdk_decoding_error",
            message: message,
            kind: "local",
            correlationId: nil,
            details: details
        )
    }

    static func localHttpStatus(status: Int, bodyPreview: String?) -> RonProblem {
        var details: [String: CodableValue] = [
            "httpStatus": .int(status)
        ]
        if let preview = bodyPreview {
            details["bodyPreview"] = .string(preview)
        }

        return RonProblem(
            code: "swift_sdk_http_status",
            message: "Non-success HTTP status: \(status)",
            kind: "local",
            correlationId: nil,
            details: details
        )
    }
}
