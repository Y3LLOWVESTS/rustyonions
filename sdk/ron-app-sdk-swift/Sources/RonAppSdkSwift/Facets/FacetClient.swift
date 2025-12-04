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

