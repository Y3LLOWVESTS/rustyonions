<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Middleware\IdempotencyMiddleware;
use Ron\AppSdkPhp\Response;

/**
 * Simple capturing client used to inspect headers passed through the middleware.
 */
final class CapturingHeadersHttpClient implements HttpClientInterface
{
    /** @var array<string,string> */
    public array $lastHeaders = [];

    public function request(
        string $method,
        string $url,
        array $headers = [],
        ?string $body = null,
        int $timeoutMs = 10_000
    ): Response {
        // Capture exactly what was sent downstream.
        $this->lastHeaders = $headers;

        return new Response(
            200,
            ['content-type' => ['application/json']],
            '{"ok":true}'
        );
    }
}

/**
 * RO:WHAT — Tests for IdempotencyMiddleware.
 * RO:WHY  — Ensure it injects keys for unsafe methods and respects existing ones.
 */
final class IdempotencyMiddlewareTest extends TestCase
{
    public function testInjectsKeyForPostWhenMissing(): void
    {
        $client = new CapturingHeadersHttpClient();

        $mw = new IdempotencyMiddleware($client);

        $mw->request(
            'POST',
            'https://example.test/api/items',
            [] // no idempotency header
        );

        $headers = $client->lastHeaders;

        $this->assertArrayHasKey('X-Idempotency-Key', $headers);
        $this->assertNotSame('', $headers['X-Idempotency-Key']);
    }

    public function testDoesNotOverrideExistingKey(): void
    {
        $client = new CapturingHeadersHttpClient();

        $mw = new IdempotencyMiddleware($client);

        $mw->request(
            'PATCH',
            'https://example.test/api/items/1',
            ['X-Idempotency-Key' => 'existing-key']
        );

        $headers = $client->lastHeaders;

        $this->assertArrayHasKey('X-Idempotency-Key', $headers);
        $this->assertSame('existing-key', $headers['X-Idempotency-Key']);
    }

    public function testDoesNotInjectForGet(): void
    {
        $client = new CapturingHeadersHttpClient();

        $mw = new IdempotencyMiddleware($client);

        $mw->request(
            'GET',
            'https://example.test/api/items',
            []
        );

        $headers = $client->lastHeaders;

        $this->assertArrayNotHasKey('X-Idempotency-Key', $headers);
    }
}
