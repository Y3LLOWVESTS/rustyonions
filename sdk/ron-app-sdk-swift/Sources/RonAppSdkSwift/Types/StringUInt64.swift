/// Wrapper for UInt64 encoded/decoded as a JSON string (canonical u64 mapping).

import Foundation

public struct StringUInt64: Codable {
    public let value: UInt64

    public init(_ value: UInt64) {
        self.value = value
    }
}

