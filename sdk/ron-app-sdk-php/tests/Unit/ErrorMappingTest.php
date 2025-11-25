<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\Exception\RonAuthException;
use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonProblemException;
use Ron\AppSdkPhp\Response;
use Ron\AppSdkPhp\RonClient;
use Ron\AppSdkPhp\Http\HttpClientInterface;

/**
 * RO:WHAT — Tests for mapping HTTP responses to Ron*Exception types.
 * RO:WHY  — Ensure auth errors, Problem envelopes, and malformed bodies
 *           become the right high-level exceptions.
 */
final class ErrorMappingTest extends TestCase
{
    public function testAuthFailuresMapToRonAuthException(): void
    {
        $fakeClient = new class () implements HttpClientInterface {
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
                return new Response(
                    401,
                    ['Content-Type' => ['application/json']],
                    '{"message":"Unauthorized"}'
                );
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.example.test',
        ])->withHttpClient($fakeClient);

        $client = new RonClient($config);

        $this->expectException(RonAuthException::class);

        $client->get('/secure');
    }

    public function testJsonProblemMapsToRonProblemException(): void
    {
        $fakeClient = new class () implements HttpClientInterface {
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
                $problem = [
                    'code'           => 'rate_limited',
                    'message'        => 'Too many requests.',
                    'kind'           => 'throttle',
                    'status'         => 429,
                    'title'          => 'Too Many Requests',
                    'correlation_id' => 'corr-123',
                ];

                return new Response(
                    429,
                    ['Content-Type' => ['application/json']],
                    \json_encode($problem, \JSON_THROW_ON_ERROR)
                );
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.example.test',
        ])->withHttpClient($fakeClient);

        $client = new RonClient($config);

        try {
            $client->get('/limit');
            $this->fail('Expected RonProblemException to be thrown.');
        } catch (RonProblemException $e) {
            $problem = $e->getProblem();

            $this->assertSame('rate_limited', $problem->getCode());
            $this->assertSame('Too many requests.', $problem->getMessage());
            $this->assertSame(429, $problem->getStatus());
            $this->assertSame('corr-123', $problem->getCorrelationId());
            $this->assertSame('Too Many Requests', $problem->getTitle());
        }
    }

    public function testMalformedJsonErrorBodyMapsToRonNetworkException(): void
    {
        $fakeClient = new class () implements HttpClientInterface {
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
                // Invalid JSON (trailing comma).
                return new Response(
                    500,
                    ['Content-Type' => ['application/json']],
                    '{"oops": "bad",}'
                );
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.example.test',
        ])->withHttpClient($fakeClient);

        $client = new RonClient($config);

        $this->expectException(RonNetworkException::class);

        $client->get('/broken');
    }

    public function testNonJsonErrorBodyMapsToRonNetworkException(): void
    {
        $fakeClient = new class () implements HttpClientInterface {
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
                return new Response(
                    502,
                    ['Content-Type' => ['text/html']],
                    '<html><body>Bad gateway</body></html>'
                );
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://gateway.example.test',
        ])->withHttpClient($fakeClient);

        $client = new RonClient($config);

        $this->expectException(RonNetworkException::class);

        $client->get('/non-json-error');
    }
}
