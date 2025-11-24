<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Http;

use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonTimeoutException;
use Ron\AppSdkPhp\Response;

/**
 * RO:WHAT — Minimal HTTP client abstraction used by RonClient.
 * RO:WHY  — Allows swapping transport (Guzzle, Symfony, custom) without changing SDK surface.
 * RO:INTERACTS — ClientConfig (optional injected client), GuzzleHttpClient, SymfonyHttpClient, RonClient.
 * RO:INVARIANTS — Must map network/timeout failures into RonNetworkException/RonTimeoutException.
 */
interface HttpClientInterface
{
    /**
     * Perform a single HTTP request.
     *
     * @param string               $method     HTTP verb (GET, POST, PUT, DELETE, ...)
     * @param string               $url        Absolute URL (including query string).
     * @param array<string,string> $headers    HTTP headers (canonicalized by implementation).
     * @param string|null          $body       Request body (already JSON-encoded if needed).
     * @param int                  $timeoutMs  Overall timeout in milliseconds.
     *
     * @throws RonNetworkException On connection errors, DNS failures, TLS issues, etc.
     * @throws RonTimeoutException On connect/read/write timeouts.
     */
    public function request(
        string $method,
        string $url,
        array $headers = [],
        ?string $body = null,
        int $timeoutMs = 10_000
    ): Response;
}
