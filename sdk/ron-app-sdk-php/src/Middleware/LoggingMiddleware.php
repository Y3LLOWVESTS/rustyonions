<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Middleware;

use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Response;

/**
 * RO:WHAT — HttpClient decorator that emits structured logs for requests.
 * RO:WHY  — Give apps/frameworks a simple hook to observe calls without
 *           leaking secrets or binding to a specific logging framework.
 * RO:INTERACTS — RonClient (via HttpClientInterface), app/framework loggers.
 * RO:INVARIANTS —
 *   * Never logs raw bodies.
 *   * Redacts Authorization/Cookie/X-Ron-* caps.
 *   * Logging callback must be side-effect-only; never throws.
 */
final class LoggingMiddleware implements HttpClientInterface
{
    private HttpClientInterface $inner;

    /**
     * PSR-3–ish logger callback.
     *
     * Signature: function (string $message, array $context = []): void
     *
     * @var callable(string,array<string,mixed>):void|null
     */
    private $logger;

    /**
     * @param callable(string,array<string,mixed>):void|null $logger
     */
    public function __construct(HttpClientInterface $inner, ?callable $logger = null)
    {
        $this->inner = $inner;
        $this->logger = $logger;
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
        $start = \microtime(true);

        $redactedHeaders = $this->redactHeaders($headers);

        if ($this->logger !== null) {
            $this->safeLog('ron_sdk.request', [
                'method'     => $method,
                'url'        => $url,
                'headers'    => $redactedHeaders,
                'timeout_ms' => $timeoutMs,
            ]);
        }

        try {
            $response = $this->inner->request($method, $url, $headers, $body, $timeoutMs);
        } catch (\Throwable $e) {
            if ($this->logger !== null) {
                $this->safeLog('ron_sdk.request_error', [
                    'method'     => $method,
                    'url'        => $url,
                    'headers'    => $redactedHeaders,
                    'timeout_ms' => $timeoutMs,
                    'error_class'=> \get_class($e),
                    'error_msg'  => $e->getMessage(),
                ]);
            }

            throw $e;
        }

        $durationMs = (int) \round((\microtime(true) - $start) * 1000);

        if ($this->logger !== null) {
            $correlationId = $response->getHeader('x-correlation-id')[0] ?? null;

            $this->safeLog('ron_sdk.response', [
                'method'        => $method,
                'url'           => $url,
                'status_code'   => $response->getStatusCode(),
                'duration_ms'   => $durationMs,
                'correlation_id'=> $correlationId,
            ]);
        }

        return $response;
    }

    /**
     * @param array<string,string> $headers
     *
     * @return array<string,string>
     */
    private function redactHeaders(array $headers): array
    {
        $sensitive = [
            'authorization',
            'cookie',
            'set-cookie',
            'x-ron-cap',
            'x-api-key',
        ];

        $out = [];

        foreach ($headers as $name => $value) {
            $lower = \strtolower($name);

            if (\in_array($lower, $sensitive, true)) {
                $out[$name] = '***redacted***';
            } else {
                $out[$name] = $value;
            }
        }

        return $out;
    }

    /**
     * @param array<string,mixed> $context
     */
    private function safeLog(string $message, array $context): void
    {
        if ($this->logger === null) {
            return;
        }

        try {
            ($this->logger)($message, $context);
        } catch (\Throwable) {
            // Logging must not break app flow; swallow logger failures.
        }
    }
}
