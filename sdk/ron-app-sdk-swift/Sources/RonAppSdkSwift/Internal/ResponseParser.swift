//
// ResponseParser.swift
// ron-app-sdk-swift
//
// RO:WHAT  — Turn (Data, HTTPURLResponse) into AppResponse<T>.
// RO:WHY   — Centralize JSON decoding + local error mapping.
// RO:INVARIANTS —
//   - Always returns an AppResponse<T>; never throws.
//   - Respects the shared AppResponse<T> schema when present.
//   - On garbage/empty bodies, returns a local RonProblem.
//

import Foundation

enum ResponseParser {
    static func parse<T: Decodable>(
        data: Data,
        response: HTTPURLResponse
    ) -> AppResponse<T> {
        let status = response.statusCode
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase

        // Happy path: 2xx and well-formed AppResponse<T>.
        if (200..<300).contains(status) {
            if !data.isEmpty {
                do {
                    let decoded = try decoder.decode(AppResponse<T>.self, from: data)
                    return decoded
                } catch {
                    let problem = RonProblem.localDecodingError(
                        status: status,
                        message: "Failed to decode AppResponse<\(T.self)>: \(error)"
                    )
                    return AppResponse(ok: false, value: nil, problem: problem)
                }
            } else {
                // 2xx but empty body; treat as ok with nil value.
                return AppResponse(ok: true, value: nil, problem: nil)
            }
        }

        // Non-2xx: try to decode AppResponse<T> first.
        if !data.isEmpty {
            if let decoded = try? decoder.decode(AppResponse<T>.self, from: data) {
                return decoded
            }

            // Or a bare RonProblem.
            if let problem = try? decoder.decode(RonProblem.self, from: data) {
                return AppResponse(ok: false, value: nil, problem: problem)
            }
        }

        // Fallback: local HTTP status-based problem with a small body preview.
        let bodyPreview: String?
        if !data.isEmpty {
            bodyPreview = String(data: data.prefix(512), encoding: .utf8)
        } else {
            bodyPreview = nil
        }

        let problem = RonProblem.localHttpStatus(
            status: status,
            bodyPreview: bodyPreview
        )
        return AppResponse(ok: false, value: nil, problem: problem)
    }
}
