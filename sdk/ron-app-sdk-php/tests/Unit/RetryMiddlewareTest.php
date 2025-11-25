<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Middleware\RetryMiddleware;
use Ron\AppSdkPhp\Response;

/**
 * RO:WHAT — Tests for RetryMiddleware.
 * RO:WHY  — Ensure bounded retries only on eligible methods and errors.
 */
final class RetryMiddlewareTest extends TestCase
{
    public function testRetriesOnNetworkErrorForGet(): void
    {
        $calls = 0;

        $inner = new class ($calls) implements HttpClientInterface {
            private int $calls;

            public function __construct(int &$calls)
            {
                $this->calls = &$calls;
            }

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                $this->calls++;

                if ($this->calls === 1) {
                    throw new RonNetworkException(
                        'Simulated network failure',
                        'simulated'
                    );
                }

                return new Response(
                    200,
                    ['content-type' => ['application/json']],
                    '{"ok":true}'
                );
            }
        };

        $mw = new RetryMiddleware($inner, maxRetries: 2, backoffMs: 0);

        $response = $mw->request(
            'GET',
            'https://example.test/api/retry'
        );

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame(2, $calls, 'Expected one retry after initial failure.');
    }

    public function testDoesNotRetryOnPostWithoutIdempotencyKey(): void
    {
        $calls = 0;

        $inner = new class ($calls) implements HttpClientInterface {
            private int $calls;

            public function __construct(int &$calls)
            {
                $this->calls = &$calls;
            }

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                $this->calls++;

                throw new RonNetworkException(
                    'Simulated network failure',
                    'simulated'
                );
            }
        };

        $mw = new RetryMiddleware($inner, maxRetries: 3, backoffMs: 0);

        $this->expectException(RonNetworkException::class);

        try {
            $mw->request(
                'POST',
                'https://example.test/api/unsafe',
                [] // no idempotency key
            );
        } finally {
            // Should only call inner once, since POST without idempotency is not eligible.
            $this->assertSame(1, $calls);
        }
    }

    public function testRetriesOnPostWithIdempotencyKey(): void
    {
        $calls = 0;

        $inner = new class ($calls) implements HttpClientInterface {
            private int $calls;

            public function __construct(int &$calls)
            {
                $this->calls = &$calls;
            }

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                $this->calls++;

                if ($this->calls < 2) {
                    throw new RonNetworkException(
                        'Simulated network failure',
                        'simulated'
                    );
                }

                return new Response(
                    200,
                    ['content-type' => ['application/json']],
                    '{"ok":true}'
                );
            }
        };

        $mw = new RetryMiddleware($inner, maxRetries: 3, backoffMs: 0);

        $response = $mw->request(
            'POST',
            'https://example.test/api/create',
            ['X-Idempotency-Key' => 'key-123']
        );

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame(2, $calls, 'Expected retry after idempotent POST failure.');
    }
}
