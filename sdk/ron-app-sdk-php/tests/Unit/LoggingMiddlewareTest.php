<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Middleware\LoggingMiddleware;
use Ron\AppSdkPhp\Response;

/**
 * RO:WHAT — Tests for LoggingMiddleware.
 * RO:WHY  — Ensure it logs scrubbed metadata and never leaks secrets.
 */
final class LoggingMiddlewareTest extends TestCase
{
    public function testLogsRequestAndResponseWithRedactedHeaders(): void
    {
        $events = [];

        $inner = new class () implements HttpClientInterface {
            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                // Echo back a simple 200 JSON response.
                return new Response(
                    200,
                    [
                        'content-type'      => ['application/json'],
                        'x-correlation-id'  => ['corr-123'],
                        'set-cookie'        => ['secret-cookie=value'],
                    ],
                    '{"ok":true}'
                );
            }
        };

        $logger = static function (array $context) use (&$events): void {
            $events[] = $context;
        };

        $middleware = new LoggingMiddleware(
            $inner,
            $logger,
            null,
            false // logBodies = false
        );

        $middleware->request(
            'GET',
            'https://example.test/api/hello',
            [
                'Authorization' => 'Bearer secret',
                'Cookie'        => 'session=abc',
                'X-Ron-Cap'     => 'internal-cap',
                'X-Custom'      => 'visible',
            ],
            null,
            5000
        );

        // We expect at least two events: request + response.
        $this->assertGreaterThanOrEqual(2, \count($events));

        $requestEvent = $events[0];
        $responseEvent = $events[\count($events) - 1];

        $this->assertSame('ron_sdk_http_request', $requestEvent['kind']);
        $this->assertSame('ron_sdk_http_response', $responseEvent['kind']);

        // Headers in request log should be arrays and sensitive values redacted.
        /** @var array<string,mixed> $loggedReqHeaders */
        $loggedReqHeaders = $requestEvent['headers'];

        $this->assertArrayHasKey('Authorization', $loggedReqHeaders);
        $this->assertArrayHasKey('Cookie', $loggedReqHeaders);
        $this->assertArrayHasKey('X-Ron-Cap', $loggedReqHeaders);
        $this->assertArrayHasKey('X-Custom', $loggedReqHeaders);

        $this->assertSame(['[REDACTED]'], $loggedReqHeaders['Authorization']);
        $this->assertSame(['[REDACTED]'], $loggedReqHeaders['Cookie']);
        $this->assertSame(['[REDACTED]'], $loggedReqHeaders['X-Ron-Cap']);
        $this->assertSame(['visible'], $loggedReqHeaders['X-Custom']);

        // Response log should include status + correlation id in headers.
        /** @var array<string,mixed> $loggedResHeaders */
        $loggedResHeaders = $responseEvent['headers'];

        $this->assertArrayHasKey('x-correlation-id', $loggedResHeaders);
        $this->assertSame(['corr-123'], $loggedResHeaders['x-correlation-id']);
        $this->assertArrayHasKey('set-cookie', $loggedResHeaders);
        $this->assertSame(['[REDACTED]'], $loggedResHeaders['set-cookie']);
    }

    public function testLoggerExceptionsAreSwallowed(): void
    {
        $inner = new class () implements HttpClientInterface {
            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                return new Response(
                    200,
                    ['content-type' => ['application/json']],
                    '{"ok":true}'
                );
            }
        };

        $logger = static function (array $context): void {
            throw new \RuntimeException('Logger failed');
        };

        $middleware = new LoggingMiddleware($inner, $logger);

        // If the logger throws, the request should still succeed.
        $response = $middleware->request(
            'GET',
            'https://example.test/api/ok'
        );

        $this->assertSame(200, $response->getStatusCode());
    }
}
