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

