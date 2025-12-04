/// RonLogger — small logging facade; can be wired to SwiftLog or custom logger.

import Foundation

public struct RonLogger {
    public init() {}

    public func debug(_ message: String) {
        // TODO: Integrate with actual logging backend.
        print("[ron-sdk-swift][DEBUG] \(message)")
    }
}

