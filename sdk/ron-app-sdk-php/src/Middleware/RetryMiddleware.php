<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Middleware;

use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonTimeoutException;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Response;

/**
 * RO:WHAT — Bounded retry wrapper for HttpClientInterface.
 * RO:WHY  — Provides reusable, policy-driven retries separate from RonClient.
 * RO:INTERACTS — HttpClientInterface, IdempotencyMiddleware, ClientConfig (future).
 * RO:INVARIANTS —
 *   * Retries are bounded (no infinite loops).
 *   * Retries are only applied to idempotent operations:
 *       * GET/HEAD always.
 *       * PUT/DELETE/POST/PATCH only when an idempotency key header is present.
 *   * Backoff is in milliseconds; 0 means “no delay”.
 *
 * NOTE: RonClient currently has its own internal retry loop; this middleware
 * is provided as a building block for advanced usage patterns and future
 * refactoring toward fully pluggable policies.
 */
final class RetryMiddleware implements HttpClientInterface
{
    private HttpClientInterface $inner;

    private int $maxRetries;

    private int $backoffMs;

    public function __construct(
        HttpClientInterface $inner,
        int $maxRetries = 0,
        int $backoffMs = 0
    ) {
        $this->inner = $inner;
        $this->maxRetries = max(0, $maxRetries);
        $this->backoffMs = max(0, $backoffMs);
    }

    /**
     * @param array<string,string> $headers
     *
     * @throws RonNetworkException
     * @throws RonTimeoutException
     */
    public function request(
        string $method,
        string $url,
        array $headers = [],
        ?string $body = null,
        int $timeoutMs = 10_000
    ): Response {
        $methodUpper = \strtoupper($method);

        // If retries are disabled or method is not eligible, just pass through.
        if ($this->maxRetries === 0 || !$this->shouldRetryMethod($methodUpper, $headers)) {
            return $this->inner->request($methodUpper, $url, $headers, $body, $timeoutMs);
        }

        $attempt = 0;

        while (true) {
            try {
                return $this->inner->request($methodUpper, $url, $headers, $body, $timeoutMs);
            } catch (RonTimeoutException | RonNetworkException $e) {
                if ($attempt >= $this->maxRetries) {
                    throw $e;
                }

                ++$attempt;

                if ($this->backoffMs > 0) {
                    \usleep($this->backoffMs * 1000);
                }

                // loop and retry
            }
        }
    }

    /**
     * @param array<string,string> $headers
     */
    private function shouldRetryMethod(string $method, array $headers): bool
    {
        // GET and HEAD are always idempotent.
        if ($method === 'GET' || $method === 'HEAD') {
            return true;
        }

        // For unsafe methods, require an explicit idempotency key.
        if (
            $method === 'POST'
            || $method === 'PUT'
            || $method === 'PATCH'
            || $method === 'DELETE'
        ) {
            return $this->hasIdempotencyKey($headers);
        }

        return false;
    }

    /**
     * @param array<string,string> $headers
     */
    private function hasIdempotencyKey(array $headers): bool
    {
        foreach ($headers as $name => $_value) {
            $lower = \strtolower((string) $name);

            if ($lower === 'x-idempotency-key') {
                return true;
            }
        }

        return false;
    }
}
