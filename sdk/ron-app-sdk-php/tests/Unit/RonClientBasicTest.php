<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Response;
use Ron\AppSdkPhp\RonClient;

/**
 * Simple capturing client for inspecting what RonClient sends over the wire.
 */
final class CapturingHttpClient implements HttpClientInterface
{
    public string $lastMethod = '';
    public string $lastUrl = '';
    /** @var array<string,string> */
    public array $lastHeaders = [];
    public ?string $lastBody = null;
    public int $lastTimeoutMs = 0;

    public function request(
        string $method,
        string $url,
        array $headers = [],
        ?string $body = null,
        int $timeoutMs = 10_000
    ): Response {
        $this->lastMethod = $method;
        $this->lastUrl = $url;
        $this->lastHeaders = $headers;
        $this->lastBody = $body;
        $this->lastTimeoutMs = $timeoutMs;

        return new Response(200, ['content-type' => ['application/json']], '{"ok":true}');
    }
}

/**
 * Fake client that fails with RonNetworkException a fixed number of times
 * before succeeding. Used to exercise RonClient's retry loop.
 */
final class FlakyNetworkHttpClient implements HttpClientInterface
{
    public int $calls = 0;
    private int $failuresBeforeSuccess;

    public function __construct(int $failuresBeforeSuccess = 1)
    {
        $this->failuresBeforeSuccess = $failuresBeforeSuccess < 0
            ? 0
            : $failuresBeforeSuccess;
    }

    public function request(
        string $method,
        string $url,
        array $headers = [],
        ?string $body = null,
        int $timeoutMs = 10_000
    ): Response {
        $this->calls++;

        if ($this->calls <= $this->failuresBeforeSuccess) {
            throw new RonNetworkException(
                'Simulated transient network error.',
                'simulated_network_error'
            );
        }

        return new Response(200, ['content-type' => ['application/json']], '{"ok":true}');
    }
}

/**
 * RO:WHAT — Basic behaviour tests for RonClient (happy-path + retries).
 * RO:WHY  — Ensure URL building, headers, JSON encoding, and retry loop behave.
 */
final class RonClientBasicTest extends TestCase
{
    public function testGetBuildsExpectedUrlAndHeaders(): void
    {
        $http = new CapturingHttpClient();

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.example.test',
            'token'   => 'test-token',
        ])->withHttpClient($http);

        $client = new RonClient($config);

        $response = $client->get('/hello', ['foo' => 'bar']);

        $this->assertTrue($response->isSuccess());
        $this->assertSame(200, $response->getStatusCode());

        $this->assertSame('GET', $http->lastMethod);
        $this->assertSame(
            'https://gateway.example.test/app/hello?foo=bar',
            $http->lastUrl
        );

        $headers = $http->lastHeaders;

        // Some required headers should be present.
        $this->assertArrayHasKey('accept', $headers);
        $this->assertArrayHasKey('user-agent', $headers);
        $this->assertArrayHasKey('authorization', $headers);
        $this->assertArrayHasKey('x-ron-sdk-name', $headers);
        $this->assertArrayHasKey('x-ron-sdk-version', $headers);

        // Auth header should be a bearer token derived from config.
        $this->assertSame('Bearer test-token', $headers['authorization']);

        // Correlation IDs are generated if not provided.
        $this->assertArrayHasKey('x-request-id', $headers);
        $this->assertArrayHasKey('x-correlation-id', $headers);
        $this->assertSame($headers['x-request-id'], $headers['x-correlation-id']);

        // No request body for GET.
        $this->assertNull($http->lastBody);
    }

    public function testPostJsonEncodesBodyAndSetsContentType(): void
    {
        $http = new CapturingHttpClient();

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.example.test/',
        ])->withHttpClient($http);

        $client = new RonClient($config);

        $payload = ['name' => 'alice', 'age' => 42];

        $response = $client->post('items', $payload);

        $this->assertSame(200, $response->getStatusCode());
        $this->assertTrue($response->isSuccess());

        $this->assertSame('POST', $http->lastMethod);
        $this->assertSame(
            'https://gateway.example.test/app/items',
            $http->lastUrl
        );

        $headers = $http->lastHeaders;

        // Content type should default to application/json when using json body.
        $this->assertArrayHasKey('content-type', $headers);
        $this->assertSame('application/json', $headers['content-type']);

        // Body should be a JSON string.
        $this->assertIsString($http->lastBody);
        $decoded = \json_decode((string) $http->lastBody, true);
        $this->assertSame($payload, $decoded);
    }

    public function testRetriesOnNetworkError(): void
    {
        $http = new FlakyNetworkHttpClient(1); // fail once, then succeed

        $config = ClientConfig::fromArray([
            'baseUrl'        => 'https://gateway.example.test',
            'maxRetries'     => 1,
            'retryBackoffMs' => 1,
        ])->withHttpClient($http);

        $client = new RonClient($config);

        $response = $client->get('/hello');

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame(
            2,
            $http->calls,
            'Expected one retry after initial RonNetworkException.'
        );
    }
}
