/// Generic Page<T> type for paginated responses.
/// Mirrors the canonical pagination envelope (items + next_page_token).

import Foundation

public struct Page<T: Decodable>: Decodable {
    public let items: [T]
    public let nextPageToken: String?
}

