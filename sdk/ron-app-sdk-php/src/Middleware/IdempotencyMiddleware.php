<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Middleware;

use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Response;

/**
 * RO:WHAT — HttpClient decorator that injects idempotency keys for writes.
 * RO:WHY  — Help apps achieve safe retries for POST/PUT/PATCH without
 *           manually managing idempotency headers.
 * RO:INTERACTS — HttpClientInterface (inner), RetryMiddleware, RonClient (future).
 * RO:INVARIANTS —
 *   * Applies only to mutating methods (POST, PUT, PATCH by default).
 *   * Respects any existing x-idempotency-key header set by the caller.
 *   * Key generation is opaque, random, collision-resistant enough for SDK use.
 */
final class IdempotencyMiddleware implements HttpClientInterface
{
    private HttpClientInterface $inner;

    /**
     * Optional custom key generator.
     *
     * Signature: function (): string
     *
     * @var callable():string|null
     */
    private $keyGenerator;

    /**
     * @param callable():string|null $keyGenerator
     */
    public function __construct(HttpClientInterface $inner, ?callable $keyGenerator = null)
    {
        $this->inner = $inner;
        $this->keyGenerator = $keyGenerator;
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
        $upperMethod = \strtoupper($method);

        if ($this->isIdempotencyCandidate($upperMethod)) {
            $normalizedHeaders = $this->normalizeHeaders($headers);

            if (!isset($normalizedHeaders['x-idempotency-key'])) {
                $key = $this->generateKey();
                // Preserve original casing as best as we can; default to lower.
                $headers['x-idempotency-key'] = $key;
            }
        }

        return $this->inner->request($method, $url, $headers, $body, $timeoutMs);
    }

    private function isIdempotencyCandidate(string $method): bool
    {
        // Conservative default: only apply to standard write methods.
        return \in_array($method, ['POST', 'PUT', 'PATCH'], true);
    }

    /**
     * @param array<string,string> $headers
     *
     * @return array<string,string>
     */
    private function normalizeHeaders(array $headers): array
    {
        $normalized = [];

        foreach ($headers as $name => $value) {
            $normalized[\strtolower($name)] = $value;
        }

        return $normalized;
    }

    private function generateKey(): string
    {
        if ($this->keyGenerator !== null) {
            return ($this->keyGenerator)();
        }

        return \bin2hex(\random_bytes(16));
    }
}
