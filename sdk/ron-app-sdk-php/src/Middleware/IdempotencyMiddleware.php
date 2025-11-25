<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Middleware;

use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Response;
use Ron\AppSdkPhp\Util\IdempotencyKey;

/**
 * RO:WHAT — Idempotency key injector for write operations.
 * RO:WHY  — Enables safe retries for non-GET/HEAD calls.
 * RO:INTERACTS — HttpClientInterface, RetryMiddleware, RonClient (future).
 * RO:INVARIANTS —
 *   * Only affects unsafe methods (POST/PUT/PATCH/DELETE).
 *   * Respects any caller-provided idempotency header.
 *   * Keys are generated via IdempotencyKey helper (opaque, bounded).
 */
final class IdempotencyMiddleware implements HttpClientInterface
{
    private HttpClientInterface $inner;

    /**
     * When true, automatically inject an idempotency key for unsafe methods
     * that do not already specify one.
     */
    private bool $autoForUnsafeMethods;

    public function __construct(
        HttpClientInterface $inner,
        bool $autoForUnsafeMethods = true
    ) {
        $this->inner = $inner;
        $this->autoForUnsafeMethods = $autoForUnsafeMethods;
    }

    /**
     * @param array<string,string> $headers
     */
    public function request(
        string $method,
        string $url,
        array $headers = [],
        ?string $body = null,
        int $timeoutMs = 10_000
    ): Response {
        $methodUpper = \strtoupper($method);

        if ($this->autoForUnsafeMethods && $this->isUnsafeMethod($methodUpper)) {
            if (!$this->hasIdempotencyKey($headers)) {
                // Generate a fresh key and inject it.
                $key = IdempotencyKey::generate()->asString();
                $headers['X-Idempotency-Key'] = $key;
            }
        }

        return $this->inner->request($methodUpper, $url, $headers, $body, $timeoutMs);
    }

    private function isUnsafeMethod(string $method): bool
    {
        return $method === 'POST'
            || $method === 'PUT'
            || $method === 'PATCH'
            || $method === 'DELETE';
    }

    /**
     * @param array<string,string> $headers
     */
    private function hasIdempotencyKey(array $headers): bool
    {
        foreach ($headers as $name => $_value) {
            if (\strtolower((string) $name) === 'x-idempotency-key') {
                return true;
            }
        }

        return false;
    }
}
