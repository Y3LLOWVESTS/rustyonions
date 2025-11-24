<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp;

use Ron\AppSdkPhp\Exception\RonAuthException;
use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonProblemException;
use Ron\AppSdkPhp\Exception\RonTimeoutException;
use Ron\AppSdkPhp\Http\GuzzleHttpClient;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Util\Json;

/**
 * RO:WHAT — Main PHP SDK client for the RON-CORE app plane.
 * RO:WHY  — Wraps HTTP, timeouts, retries, and error envelopes into a
 *           small, ergonomic API for apps.
 * RO:INTERACTS — ClientConfig, HttpClientInterface, Response, Problem.
 * RO:INVARIANTS —
 *   * Never leaks secrets in exception messages.
 *   * Non-2xx results are mapped to typed Ron*Exception subclasses.
 */
final class RonClient
{
    private const SDK_NAME = 'ron-app-sdk-php';
    private const SDK_VERSION = '0.1.0-dev';

    private ClientConfig $config;
    private HttpClientInterface $httpClient;

    /**
     * @param ClientConfig|array<string,mixed> $config
     */
    public function __construct(ClientConfig|array $config)
    {
        if (\is_array($config)) {
            $config = ClientConfig::fromArray($config);
        }

        $this->config = $config;

        $client = $config->getHttpClient();
        if ($client === null) {
            $client = new GuzzleHttpClient();
        }

        $this->httpClient = $client;
    }

    public static function fromEnv(): self
    {
        return new self(ClientConfig::fromEnv());
    }

    public function getConfig(): ClientConfig
    {
        return $this->config;
    }

    /**
     * Convenience GET wrapper.
     *
     * @param array<string,mixed> $query
     * @param array<string,string> $headers
     */
    public function get(string $path, array $query = [], array $headers = []): Response
    {
        return $this->request('GET', $path, [
            'query'   => $query,
            'headers' => $headers,
        ]);
    }

    /**
     * Convenience POST wrapper with optional JSON body.
     *
     * @param mixed                $body
     * @param array<string,string> $headers
     */
    public function post(string $path, mixed $body = null, array $headers = []): Response
    {
        return $this->request('POST', $path, [
            'json'    => $body,
            'headers' => $headers,
        ]);
    }

    /**
     * Convenience PUT wrapper with optional JSON body.
     *
     * @param mixed                $body
     * @param array<string,string> $headers
     */
    public function put(string $path, mixed $body = null, array $headers = []): Response
    {
        return $this->request('PUT', $path, [
            'json'    => $body,
            'headers' => $headers,
        ]);
    }

    /**
     * Convenience DELETE wrapper.
     *
     * @param array<string,mixed>  $query
     * @param array<string,string> $headers
     */
    public function delete(string $path, array $query = [], array $headers = []): Response
    {
        return $this->request('DELETE', $path, [
            'query'   => $query,
            'headers' => $headers,
        ]);
    }

    /**
     * Low-level request API.
     *
     * Options:
     *  - query:   array<string,mixed>
     *  - json:    mixed (will be JSON-encoded)
     *  - body:    string (raw body; ignored if json set)
     *  - headers: array<string,string>
     *
     * @param array<string,mixed> $options
     *
     * @throws RonAuthException
     * @throws RonNetworkException
     * @throws RonProblemException
     * @throws RonTimeoutException
     */
    public function request(string $method, string $path, array $options = []): Response
    {
        $method = strtoupper($method);

        /** @var array<string,mixed> $query */
        $query = isset($options['query']) && \is_array($options['query']) ? $options['query'] : [];
        /** @var array<string,string> $headers */
        $headers = isset($options['headers']) && \is_array($options['headers']) ? $options['headers'] : [];
        $body = null;

        if (\array_key_exists('json', $options)) {
            $body = Json::encode($options['json']);
            if (!isset($headers['content-type'])) {
                $headers['content-type'] = 'application/json';
            }
        } elseif (\array_key_exists('body', $options) && $options['body'] !== null) {
            $body = (string) $options['body'];
        }

        $url = $this->buildUrl($path, $query);
        $headers = $this->buildHeaders($headers);

        $timeoutMs = $this->config->getOverallTimeoutMs();
        $maxRetries = $this->config->getMaxRetries();
        $backoffMs = $this->config->getRetryBackoffMs();

        $attempt = 0;
        $response = null;

        while (true) {
            try {
                $response = $this->httpClient->request(
                    $method,
                    $url,
                    $headers,
                    $body,
                    $timeoutMs
                );

                break;
            } catch (RonTimeoutException|RonNetworkException $e) {
                if ($attempt >= $maxRetries) {
                    throw $e;
                }

                ++$attempt;

                if ($backoffMs > 0) {
                    \usleep($backoffMs * 1000);
                }

                // retry
            }
        }

        if ($response->isSuccess()) {
            return $response;
        }

        // Map non-2xx responses to a typed exception.
        $this->throwForErrorResponse($response);

        // Unreachable, but keeps static analysers happy.
        return $response;
    }

    /**
     * No-op for now; kept for API symmetry with other SDKs.
     */
    public function close(): void
    {
        // If we ever switch to a client that needs explicit shutdown, wire it here.
    }

    /**
     * @param array<string,mixed> $query
     */
    private function buildUrl(string $path, array $query): string
    {
        $base = rtrim($this->config->getBaseUrl(), '/');
        $trimmed = trim($path);

        if ($trimmed === '') {
            $relative = '/app';
        } elseif (\str_starts_with($trimmed, '/app/')) {
            // Allow callers to pass full /app/* paths (as in README examples).
            $relative = $trimmed;
        } else {
            $relative = $trimmed[0] === '/'
                ? '/app' . $trimmed
                : '/app/' . $trimmed;
        }

        $url = $base . $relative;

        if ($query !== []) {
            $qs = \http_build_query($query);
            if ($qs !== '') {
                $url .= '?' . $qs;
            }
        }

        return $url;
    }

    /**
     * @param array<string,string> $userHeaders
     *
     * @return array<string,string>
     */
    private function buildHeaders(array $userHeaders): array
    {
        $headers = [];

        // Normalize to lowercase keys.
        foreach ($userHeaders as $name => $value) {
            $headers[\strtolower($name)] = $value;
        }

        $headers['accept'] = $headers['accept'] ?? 'application/json, application/problem+json';

        $headers['user-agent'] = $headers['user-agent']
            ?? sprintf('%s/%s', self::SDK_NAME, self::SDK_VERSION);

        $headers['x-ron-sdk-lang'] = $headers['x-ron-sdk-lang'] ?? 'php';
        $headers['x-ron-sdk-name'] = $headers['x-ron-sdk-name'] ?? self::SDK_NAME;
        $headers['x-ron-sdk-version'] = $headers['x-ron-sdk-version'] ?? self::SDK_VERSION;

        // Correlation ID for tracing; if caller provided one, respect it.
        if (!isset($headers['x-request-id']) && !isset($headers['x-correlation-id'])) {
            $requestId = \bin2hex(\random_bytes(16));
            $headers['x-request-id'] = $requestId;
            $headers['x-correlation-id'] = $requestId;
        }

        // Authorization header (bearer token), if configured.
        $token = $this->config->getToken();
        if ($token !== null && $token !== '' && !isset($headers['authorization'])) {
            $headers['authorization'] = 'Bearer ' . $token;
        }

        return $headers;
    }

    /**
     * @throws RonProblemException
     * @throws RonNetworkException
     * @throws RonAuthException
     */
    private function throwForErrorResponse(Response $response): void
    {
        $status = $response->getStatusCode();
        $contentTypes = $response->getHeader('content-type');
        $contentType = $contentTypes[0] ?? '';
        $contentTypeLower = \strtolower($contentType);
        $correlationId = $response->getHeader('x-correlation-id')[0] ?? null;

        // Auth failures get a dedicated exception type.
        if ($status === 401 || $status === 403) {
            throw new RonAuthException(
                sprintf('Authentication/authorization failed (HTTP %d).', $status),
                'auth_failed',
                $status,
                $correlationId
            );
        }

        // Try to interpret as canonical problem / RFC7807.
        if (\str_contains($contentTypeLower, 'application/json')) {
            try {
                $data = $response->json(true);
            } catch (\Throwable $e) {
                throw new RonNetworkException(
                    'Received malformed JSON error response from RON-CORE gateway.',
                    'malformed_error_body',
                    $status,
                    $correlationId,
                    null,
                    $e
                );
            }

            if (\is_array($data)) {
                if (!isset($data['status'])) {
                    $data['status'] = $status;
                }

                $problem = Problem::fromArray($data);

                throw new RonProblemException($problem);
            }

            throw new RonNetworkException(
                'Received non-object JSON error response from RON-CORE gateway.',
                'malformed_error_body',
                $status,
                $correlationId
            );
        }

        // Fallback: non-JSON error; we keep the message generic.
        throw new RonNetworkException(
            sprintf('Received non-JSON error response from RON-CORE gateway (HTTP %d).', $status),
            'non_json_error',
            $status,
            $correlationId
        );
    }
}
