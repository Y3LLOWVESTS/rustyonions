package dev.roncore.sdk;

import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Collections;
import java.util.List;
import java.util.Objects;

/**
 * RO:WHAT —
 *   Generic pagination envelope type with items and nextPageToken.
 *
 * RO:WHY —
 *   Mirrors the shared pagination schema so JVM apps can page through
 *   RON resources in a uniform way.
 *
 * RO:INTERACTS —
 *   - Used as a data payload in {@link AppResponse} for list endpoints.
 *
 * RO:INVARIANTS —
 *   - {@code items} is never null; defaults to an empty list if omitted.
 */
public final class Page<T> {

    @JsonProperty("items")
    private final List<T> items;

    @JsonProperty("next_page_token")
    private final String nextPageToken;

    public Page(
            @JsonProperty("items") List<T> items,
            @JsonProperty("next_page_token") String nextPageToken
    ) {
        this.items = items != null ? List.copyOf(items) : Collections.emptyList();
        this.nextPageToken = nextPageToken;
    }

    public List<T> getItems() {
        return items;
    }

    public String getNextPageToken() {
        return nextPageToken;
    }

    @Override
    public boolean equals(Object obj) {
        if (!(obj instanceof Page<?> other)) {
            return false;
        }
        return Objects.equals(items, other.items)
                && Objects.equals(nextPageToken, other.nextPageToken);
    }

    @Override
    public int hashCode() {
        return Objects.hash(items, nextPageToken);
    }

    @Override
    public String toString() {
        return "Page{items=" + items + ", nextPageToken='" + nextPageToken + "'}";
    }
}
