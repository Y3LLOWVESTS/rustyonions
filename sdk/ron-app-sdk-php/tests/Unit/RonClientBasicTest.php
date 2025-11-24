<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Response;
use Ron\AppSdkPhp\RonClient;
use Ron\AppSdkPhp\Util\Json;

/**
 * RO:WHAT — Unit tests for basic RonClient behaviour (URLs, JSON, query).
 * RO:WHY  — Ensure we build /app URLs correctly and encode bodies as expected.
 */
final class RonClientBasicTest extends TestCase
{
    public function testGetBuildsAppUrlVariants(): void
    {
        $http = new class implements HttpClientInterface {
            public string $lastMethod = '';
            public string $lastUrl = '';
            /** @var array<string,string> */
            public array $lastHeaders = [];
            public ?string $lastBody = null;

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

                return new Response(200, [], '');
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.test',
            'httpClient' => $http,
        ]);

        $client = new RonClient($config);

        // "hello" → /app/hello
        $client->get('hello');
        $this->assertSame('GET', $http->lastMethod);
        $this->assertSame('https://gateway.test/app/hello', $http->lastUrl);

        // "/hello" → /app/hello
        $client->get('/hello');
        $this->assertSame('https://gateway.test/app/hello', $http->lastUrl);

        // "/app/hello" is passed through as-is.
        $client->get('/app/hello');
        $this->assertSame('https://gateway.test/app/hello', $http->lastUrl);
    }

    public function testGetAttachesQueryParameters(): void
    {
        $http = new class implements HttpClientInterface {
            public string $lastUrl = '';

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                $this->lastUrl = $url;

                return new Response(200, [], '');
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.test',
            'httpClient' => $http,
        ]);

        $client = new RonClient($config);

        $client->get('search', [
            'page' => 1,
            'tags' => ['rust', 'php'],
        ]);

        $this->assertStringStartsWith('https://gateway.test/app/search?', $http->lastUrl);
        $this->assertStringContainsString('page=1', $http->lastUrl);
        $this->assertStringContainsString('tags%5B0%5D=rust', $http->lastUrl);
        $this->assertStringContainsString('tags%5B1%5D=php', $http->lastUrl);
    }

    public function testPostJsonEncodesBodyAndSetsContentType(): void
    {
        $http = new class implements HttpClientInterface {
            /** @var array<string,string> */
            public array $lastHeaders = [];
            public ?string $lastBody = null;

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                $this->lastHeaders = $headers;
                $this->lastBody = $body;

                return new Response(201, ['content-type' => ['application/json']], '{"ok":true}');
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.test',
            'httpClient' => $http,
        ]);

        $client = new RonClient($config);

        $payload = ['foo' => 'bar'];

        $client->post('/items', $payload);

        $this->assertSame(
            Json::encode($payload),
            $http->lastBody
        );

        $this->assertArrayHasKey('content-type', $http->lastHeaders);
        $this->assertSame('application/json', $http->lastHeaders['content-type']);
    }
}
