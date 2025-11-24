<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Response;
use Ron\AppSdkPhp\RonClient;

/**
 * RO:WHAT — Unit tests for security / metadata headers.
 * RO:WHY  — Ensure RonClient sets SDK + auth + correlation headers correctly.
 */
final class SecurityHeadersTest extends TestCase
{
    public function testSdkAndCorrelationHeadersAreSet(): void
    {
        $http = new class implements HttpClientInterface {
            /** @var array<string,string> */
            public array $lastHeaders = [];

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                $this->lastHeaders = $headers;

                return new Response(200, [], '');
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.test',
            'token' => 'test-token',
            'httpClient' => $http,
        ]);

        $client = new RonClient($config);

        $client->get('/hello');

        $headers = $http->lastHeaders;

        // Header keys are normalized to lowercase in RonClient.
        $this->assertArrayHasKey('accept', $headers);
        $this->assertStringContainsString('application/json', $headers['accept']);

        $this->assertArrayHasKey('user-agent', $headers);
        $this->assertStringContainsString('ron-app-sdk-php', $headers['user-agent']);

        $this->assertSame('php', $headers['x-ron-sdk-lang'] ?? null);
        $this->assertSame('ron-app-sdk-php', $headers['x-ron-sdk-name'] ?? null);
        $this->assertNotSame('', $headers['x-ron-sdk-version'] ?? '');

        $this->assertArrayHasKey('x-request-id', $headers);
        $this->assertArrayHasKey('x-correlation-id', $headers);
        $this->assertNotSame('', $headers['x-request-id']);
        $this->assertNotSame('', $headers['x-correlation-id']);
        $this->assertSame($headers['x-request-id'], $headers['x-correlation-id']);

        // Authorization header should use the configured token.
        $this->assertArrayHasKey('authorization', $headers);
        $this->assertSame('Bearer test-token', $headers['authorization']);
    }

    public function testCallerCanOverrideUserAgent(): void
    {
        $http = new class implements HttpClientInterface {
            /** @var array<string,string> */
            public array $lastHeaders = [];

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                $this->lastHeaders = $headers;

                return new Response(200, [], '');
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.test',
            'httpClient' => $http,
        ]);

        $client = new RonClient($config);

        $client->get('/hello', [], [
            'User-Agent' => 'CustomAgent/1.0',
        ]);

        $headers = $http->lastHeaders;

        $this->assertSame('CustomAgent/1.0', $headers['user-agent'] ?? null);
    }
}
