<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\Exception\RonAuthException;
use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonProblemException;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Response;
use Ron\AppSdkPhp\RonClient;
use Ron\AppSdkPhp\Util\Json;

/**
 * RO:WHAT — Unit tests for RonClient error mapping behaviour.
 * RO:WHY  — Ensure non-2xx responses map into the correct Ron*Exception types.
 */
final class ErrorMappingTest extends TestCase
{
    public function testSuccessResponsePassesThrough(): void
    {
        $payload = ['ok' => true];

        $fakeResponse = new Response(
            200,
            ['content-type' => ['application/json']],
            Json::encode($payload)
        );

        $http = new class($fakeResponse) implements HttpClientInterface {
            private Response $response;

            public function __construct(Response $response)
            {
                $this->response = $response;
            }

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                return $this->response;
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://example.test',
            'httpClient' => $http,
        ]);

        $client = new RonClient($config);

        $response = $client->get('/hello');

        $this->assertSame(200, $response->getStatusCode());
        $this->assertSame($payload, $response->json(true));
    }

    public function testProblemJsonMapsToRonProblemException(): void
    {
        $problemPayload = [
            'code' => 'bad_request',
            'message' => 'The request was invalid.',
            'kind' => 'client',
            'status' => 400,
            'correlation_id' => 'corr-123',
            'details' => ['field' => 'name'],
        ];

        $fakeResponse = new Response(
            400,
            ['content-type' => ['application/json']],
            Json::encode($problemPayload)
        );

        $http = new class($fakeResponse) implements HttpClientInterface {
            private Response $response;

            public function __construct(Response $response)
            {
                $this->response = $response;
            }

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                return $this->response;
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://example.test',
            'httpClient' => $http,
        ]);

        $client = new RonClient($config);

        try {
            $client->get('/bad');
            $this->fail('Expected RonProblemException to be thrown.');
        } catch (RonProblemException $e) {
            $problem = $e->getProblem();

            $this->assertSame('bad_request', $problem->getCode());
            $this->assertSame(400, $problem->getStatus());
            $this->assertSame('corr-123', $problem->getCorrelationId());
            $this->assertSame('client', $problem->getKind());
            $this->assertSame(
                ['field' => 'name'],
                $problem->getDetails()
            );
            $this->assertSame(
                $problem->getCanonicalMessage(),
                $e->getCanonicalMessage()
            );
        }
    }

    public function testMalformedJsonErrorMapsToRonNetworkException(): void
    {
        $fakeResponse = new Response(
            500,
            ['content-type' => ['application/json']],
            '{not valid json'
        );

        $http = new class($fakeResponse) implements HttpClientInterface {
            private Response $response;

            public function __construct(Response $response)
            {
                $this->response = $response;
            }

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                return $this->response;
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://example.test',
            'httpClient' => $http,
        ]);

        $client = new RonClient($config);

        try {
            $client->get('/oops');
            $this->fail('Expected RonNetworkException to be thrown.');
        } catch (RonNetworkException $e) {
            $this->assertSame('malformed_error_body', $e->getErrorCode());
            $this->assertSame(500, $e->getStatusCode());
        }
    }

    public function testNonJsonErrorMapsToRonNetworkException(): void
    {
        $fakeResponse = new Response(
            502,
            ['content-type' => ['text/html']],
            '<html>Bad gateway</html>'
        );

        $http = new class($fakeResponse) implements HttpClientInterface {
            private Response $response;

            public function __construct(Response $response)
            {
                $this->response = $response;
            }

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                return $this->response;
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://example.test',
            'httpClient' => $http,
        ]);

        $client = new RonClient($config);

        try {
            $client->get('/gateway');
            $this->fail('Expected RonNetworkException to be thrown.');
        } catch (RonNetworkException $e) {
            $this->assertSame('non_json_error', $e->getErrorCode());
            $this->assertSame(502, $e->getStatusCode());
        }
    }

    public function testAuthErrorsMapToRonAuthException(): void
    {
        $fakeResponse = new Response(
            401,
            ['content-type' => ['application/json']],
            Json::encode(['message' => 'unauthorized'])
        );

        $http = new class($fakeResponse) implements HttpClientInterface {
            private Response $response;

            public function __construct(Response $response)
            {
                $this->response = $response;
            }

            public function request(
                string $method,
                string $url,
                array $headers = [],
                ?string $body = null,
                int $timeoutMs = 10_000
            ): Response {
                return $this->response;
            }
        };

        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://example.test',
            'httpClient' => $http,
            'token' => 'dummy-token',
        ]);

        $client = new RonClient($config);

        $this->expectException(RonAuthException::class);

        $client->get('/secure');
    }
}
