/// Wrapper for Int64 encoded/decoded as a JSON string (canonical i64 mapping).

import Foundation

public struct StringInt64: Codable {
    public let value: Int64

    public init(_ value: Int64) {
        self.value = value
    }
}

