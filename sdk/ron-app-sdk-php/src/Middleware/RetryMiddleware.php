<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Middleware;

use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonTimeoutException;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Response;

/**
 * RO:WHAT — HttpClient decorator that adds retry/backoff on transient errors.
 * RO:WHY  — Allow centralized retry policy (alternative to RonClient’s
 *           built-in retry loop, or for apps that use HttpClientInterface
 *           directly).
 * RO:INTERACTS — HttpClientInterface (inner), RonTimeoutException, RonNetworkException.
 * RO:INVARIANTS —
 *   * Retries only on RonTimeoutException / RonNetworkException.
 *   * Never retries on non-exception HTTP responses (4xx/5xx are returned).
 *   * Backoff uses simple fixed delay (ms).
 */
final class RetryMiddleware implements HttpClientInterface
{
    private HttpClientInterface $inner;
    private int $maxRetries;
    private int $backoffMs;

    public function __construct(HttpClientInterface $inner, int $maxRetries = 0, int $backoffMs = 0)
    {
        $this->inner = $inner;
        $this->maxRetries = $maxRetries < 0 ? 0 : $maxRetries;
        $this->backoffMs = $backoffMs < 0 ? 0 : $backoffMs;
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
        $attempt = 0;

        while (true) {
            try {
                return $this->inner->request($method, $url, $headers, $body, $timeoutMs);
            } catch (RonTimeoutException|RonNetworkException $e) {
                if ($attempt >= $this->maxRetries) {
                    throw $e;
                }

                ++$attempt;

                if ($this->backoffMs > 0) {
                    \usleep($this->backoffMs * 1000);
                }

                // loop again
            }
        }
    }
}
