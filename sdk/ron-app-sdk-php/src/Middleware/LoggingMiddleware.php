<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Middleware;

use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonTimeoutException;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Response;

/**
 * RO:WHAT — Logging wrapper for HttpClientInterface.
 * RO:WHY  — Emits safe, scrubbed request/response metadata for observability.
 * RO:INTERACTS — HttpClientInterface, RonClient (future opt-in), app loggers.
 * RO:INVARIANTS —
 *   * Never logs raw Authorization/cookie/X-Ron-* headers.
 *   * Bodies are NOT logged by default (opt-in with scrubber).
 *   * Safe even if logger throws (logging failure must not break app code).
 */
final class LoggingMiddleware implements HttpClientInterface
{
    private HttpClientInterface $inner;

    /**
     * @var callable|null
     *
     * Signature: fn(array<string,mixed> $context): void
     */
    private $logger;

    /**
     * @var callable|null
     *
     * Signature: fn(string $body): mixed
     * Should return a scrubbed representation (string/array) safe to log.
     */
    private $bodyScrubber;

    private bool $logBodies;

    /**
     * Header names which MUST be redacted (case-insensitive).
     *
     * @var string[]
     */
    private array $redactedHeaderNames = [
        'authorization',
        'cookie',
        'set-cookie',
    ];

    public function __construct(
        HttpClientInterface $inner,
        ?callable $logger = null,
        ?callable $bodyScrubber = null,
        bool $logBodies = false
    ) {
        $this->inner = $inner;
        $this->logger = $logger;
        $this->bodyScrubber = $bodyScrubber;
        $this->logBodies = $logBodies;
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
        $start = \microtime(true);

        $this->logRequestSafe($methodUpper, $url, $headers, $body);

        try {
            $response = $this->inner->request($methodUpper, $url, $headers, $body, $timeoutMs);
        } catch (RonTimeoutException | RonNetworkException $e) {
            $durationMs = (int) \round((\microtime(true) - $start) * 1000);

            $this->logEventSafe([
                'kind'        => 'ron_sdk_http_error',
                'method'      => $methodUpper,
                'url'         => $url,
                'duration_ms' => $durationMs,
                'error_class' => $e::class,
                'error_msg'   => $e->getMessage(),
            ]);

            throw $e;
        } catch (\Throwable $e) {
            // Defensive: log unexpected throw, then rethrow.
            $durationMs = (int) \round((\microtime(true) - $start) * 1000);

            $this->logEventSafe([
                'kind'        => 'ron_sdk_http_unexpected_error',
                'method'      => $methodUpper,
                'url'         => $url,
                'duration_ms' => $durationMs,
                'error_class' => $e::class,
                'error_msg'   => $e->getMessage(),
            ]);

            throw $e;
        }

        $durationMs = (int) \round((\microtime(true) - $start) * 1000);

        $this->logResponseSafe($methodUpper, $url, $response, $durationMs);

        return $response;
    }

    /**
     * @param array<string,string> $headers
     */
    private function logRequestSafe(
        string $method,
        string $url,
        array $headers,
        ?string $body
    ): void {
        $logHeaders = $this->scrubHeaders($headers);

        $context = [
            'kind'     => 'ron_sdk_http_request',
            'method'   => $method,
            'url'      => $url,
            'headers'  => $logHeaders,
            'has_body' => $body !== null && $body !== '',
        ];

        if ($this->logBodies) {
            $context['body'] = $this->scrubBodyForLogging($body);
        }

        $this->logEventSafe($context);
    }

    private function logResponseSafe(
        string $method,
        string $url,
        Response $response,
        int $durationMs
    ): void {
        $logHeaders = $this->scrubHeaders($response->getHeaders());

        $context = [
            'kind'        => 'ron_sdk_http_response',
            'method'      => $method,
            'url'         => $url,
            'status'      => $response->getStatusCode(),
            'duration_ms' => $durationMs,
            'headers'     => $logHeaders,
        ];

        // Body not logged by default; reserved for future opt-in.
        $this->logEventSafe($context);
    }

    /**
     * @param array<string,string>|array<string,string[]> $headers
     *
     * @return array<string,mixed>
     */
    private function scrubHeaders(array $headers): array
    {
        $out = [];

        foreach ($headers as $name => $value) {
            $lower = \strtolower((string) $name);

            $shouldRedact = $this->isRedactedHeader($lower);

            if (\is_array($value)) {
                $values = array_values($value);
            } else {
                $values = [$value];
            }

            if ($shouldRedact) {
                $out[$name] = array_fill(0, \count($values), '[REDACTED]');
                continue;
            }

            $out[$name] = $values;
        }

        return $out;
    }

    private function isRedactedHeader(string $lowerName): bool
    {
        if (\in_array($lowerName, $this->redactedHeaderNames, true)) {
            return true;
        }

        // Any X-Ron-* header is redacted by default.
        if (\str_starts_with($lowerName, 'x-ron-')) {
            return true;
        }

        return false;
    }

    private function scrubBodyForLogging(?string $body): mixed
    {
        if ($body === null || $body === '') {
            return null;
        }

        if ($this->bodyScrubber !== null) {
            try {
                /** @var callable $scrubber */
                $scrubber = $this->bodyScrubber;

                return $scrubber($body);
            } catch (\Throwable) {
                // If scrubber fails, fall back to placeholder.
                return '[BODY SCRUB FAILED]';
            }
        }

        // Even when body logging is enabled, never log raw body without scrubber.
        return '[BODY SUPPRESSED]';
    }

    /**
     * @param array<string,mixed> $context
     */
    private function logEventSafe(array $context): void
    {
        if ($this->logger === null) {
            return;
        }

        try {
            /** @var callable $logger */
            $logger = $this->logger;
            $logger($context);
        } catch (\Throwable) {
            // Logging must never break the app; swallow.
        }
    }
}
