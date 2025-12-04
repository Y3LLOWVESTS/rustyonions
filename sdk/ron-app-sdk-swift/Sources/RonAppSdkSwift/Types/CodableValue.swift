//
// CodableValue.swift
// ron-app-sdk-swift
//
// RO:WHAT  — Loosely-typed JSON value used in RonProblem.details.
// RO:WHY   — Match the shared SDK schema (bool/number/string/array/object).
//

import Foundation

public enum CodableValue: Codable, Equatable {
    case string(String)
    case int(Int)
    case double(Double)
    case bool(Bool)
    case array([CodableValue])
    case object([String: CodableValue])

    // MARK: - Convenience constructors

    public static func from(_ value: String) -> CodableValue { .string(value) }
    public static func from(_ value: Int) -> CodableValue { .int(value) }
    public static func from(_ value: Double) -> CodableValue { .double(value) }
    public static func from(_ value: Bool) -> CodableValue { .bool(value) }

    // MARK: - Codable

    public init(from decoder: Decoder) throws {
        let container = try decoder.singleValueContainer()

        if let v = try? container.decode(Bool.self) {
            self = .bool(v)
            return
        }
        if let v = try? container.decode(Int.self) {
            self = .int(v)
            return
        }
        if let v = try? container.decode(Double.self) {
            self = .double(v)
            return
        }
        if let v = try? container.decode(String.self) {
            self = .string(v)
            return
        }
        if let v = try? container.decode([CodableValue].self) {
            self = .array(v)
            return
        }
        if let v = try? container.decode([String: CodableValue].self) {
            self = .object(v)
            return
        }

        throw DecodingError.dataCorruptedError(
            in: container,
            debugDescription: "Unsupported JSON value for CodableValue"
        )
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.singleValueContainer()

        switch self {
        case .bool(let v):
            try container.encode(v)
        case .int(let v):
            try container.encode(v)
        case .double(let v):
            try container.encode(v)
        case .string(let v):
            try container.encode(v)
        case .array(let arr):
            try container.encode(arr)
        case .object(let obj):
            try container.encode(obj)
        }
    }
}
