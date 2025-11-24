<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp;

/**
 * RO:WHAT — Value object for a single paginated response.
 * RO:WHY  — Wraps `{ items, next_page_token }` into a typed helper.
 * RO:INVARIANTS —
 *   - Items is always an array (may be empty).
 *   - nextPageToken is null or a non-empty string.
 *
 * @template T
 */
final class Page
{
    /** @var list<T> */
    private array $items;

    private ?string $nextPageToken;

    /**
     * @param list<T> $items
     */
    public function __construct(array $items, ?string $nextPageToken)
    {
        $this->items = $items;
        $this->nextPageToken = $nextPageToken;
    }

    /**
     * @return list<T>
     */
    public function getItems(): array
    {
        return $this->items;
    }

    public function getNextPageToken(): ?string
    {
        return $this->nextPageToken;
    }
}

/**
 * RO:WHAT — Lazy paginator helper over `Page<T>` sequences.
 * RO:WHY  — Encodes canonical `{ items, next_page_token }` envelope into a simple iterator.
 * RO:INVARIANTS —
 *   - Never infinite: stops when `next_page_token` is null/empty.
 *   - Does not unboundedly buffer pages; yields items as they arrive.
 */
final class Paginator
{
    /**
     * Iterate lazily across pages using a callback that fetches each `Page<T>`.
     *
     * Typical usage (pseudocode):
     *
     * ```php
     * $items = [];
     * foreach (Paginator::iterate($fetchPage) as $item) {
     *     $items[] = $item;
     * }
     * ```
     *
     * @template T
     *
     * @param callable(?string): Page<T> $fetchPage
     *        Callback that accepts the current page token (or null for first page)
     *        and returns a `Page<T>` with items and the next page token.
     *
     * @return \Generator<int, T, void, void>
     */
    public static function iterate(callable $fetchPage): \Generator
    {
        $token = null;

        while (true) {
            /** @var Page<T> $page */
            $page = $fetchPage($token);

            $items = $page->getItems();

            foreach ($items as $item) {
                yield $item;
            }

            $token = $page->getNextPageToken();

            // Stop when no further page token is provided.
            if ($token === null || $token === '') {
                break;
            }
        }
    }
}
