<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp;

use Ron\AppSdkPhp\Exception\RonConfigException;
use Ron\AppSdkPhp\Http\HttpClientInterface;
use Ron\AppSdkPhp\Util\Env;

/**
 * RO:WHAT — SDK configuration value object.
 * RO:WHY — Centralizes env/config parsing and validation.
 * RO:INTERACTS — RonClient, HttpClientInterface.
 * RO:INVARIANTS — Fail-closed on invalid URLs/timeouts; safe defaults.
 */
final class ClientConfig
{
    private string $baseUrl;
    private ?string $token;
    private bool $allowInsecureHttp;

    private int $connectTimeoutMs;
    private int $readTimeoutMs;
    private int $writeTimeoutMs;
    private int $overallTimeoutMs;

    private int $maxRetries;
    private int $retryBackoffMs;

    private ?HttpClientInterface $httpClient;

    public function __construct(
        string $baseUrl,
        ?string $token = null,
        bool $allowInsecureHttp = false,
        int $connectTimeoutMs = 1_000,
        int $readTimeoutMs = 5_000,
        int $writeTimeoutMs = 5_000,
        int $overallTimeoutMs = 10_000,
        int $maxRetries = 0,
        int $retryBackoffMs = 100,
        ?HttpClientInterface $httpClient = null
    ) {
        $this->baseUrl = self::normalizeBaseUrl($baseUrl, $allowInsecureHttp);
        $this->token = $token;
        $this->allowInsecureHttp = $allowInsecureHttp;

        $this->connectTimeoutMs = self::validatePositive('connectTimeoutMs', $connectTimeoutMs);
        $this->readTimeoutMs = self::validatePositive('readTimeoutMs', $readTimeoutMs);
        $this->writeTimeoutMs = self::validatePositive('writeTimeoutMs', $writeTimeoutMs);
        $this->overallTimeoutMs = self::validatePositive('overallTimeoutMs', $overallTimeoutMs);

        if ($maxRetries < 0) {
            throw new RonConfigException('maxRetries must not be negative.');
        }

        if ($retryBackoffMs < 0) {
            throw new RonConfigException('retryBackoffMs must not be negative.');
        }

        $this->maxRetries = $maxRetries;
        $this->retryBackoffMs = $retryBackoffMs;
        $this->httpClient = $httpClient;
    }

    /**
     * Build a ClientConfig from the shared env schema.
     *
     * Required:
     *   - RON_SDK_GATEWAY_ADDR
     *
     * Optional:
     *   - RON_SDK_TOKEN
     *   - RON_SDK_INSECURE_HTTP (bool)
     *   - RON_SDK_CONNECT_TIMEOUT_MS
     *   - RON_SDK_READ_TIMEOUT_MS
     *   - RON_SDK_WRITE_TIMEOUT_MS
     *   - RON_SDK_OVERALL_TIMEOUT_MS
     *
     * @throws RonConfigException
     */
    public static function fromEnv(): self
    {
        $baseUrl = Env::getString('RON_SDK_GATEWAY_ADDR');
        if ($baseUrl === null) {
            throw new RonConfigException('RON_SDK_GATEWAY_ADDR is required.');
        }

        $token = Env::getString('RON_SDK_TOKEN');
        $allowInsecure = Env::getBool('RON_SDK_INSECURE_HTTP', false) ?? false;

        $connectTimeoutMs = Env::getInt('RON_SDK_CONNECT_TIMEOUT_MS', 1_000) ?? 1_000;
        $readTimeoutMs = Env::getInt('RON_SDK_READ_TIMEOUT_MS', 5_000) ?? 5_000;
        $writeTimeoutMs = Env::getInt('RON_SDK_WRITE_TIMEOUT_MS', 5_000) ?? 5_000;
        $overallTimeoutMs = Env::getInt('RON_SDK_OVERALL_TIMEOUT_MS', 10_000) ?? 10_000;

        return new self(
            $baseUrl,
            $token,
            $allowInsecure,
            $connectTimeoutMs,
            $readTimeoutMs,
            $writeTimeoutMs,
            $overallTimeoutMs
        );
    }

    /**
     * @param array<string,mixed> $options
     *
     * @throws RonConfigException
     */
    public static function fromArray(array $options): self
    {
        $baseUrl = $options['baseUrl'] ?? $options['base_url'] ?? null;
        if (!\is_string($baseUrl) || $baseUrl === '') {
            throw new RonConfigException('baseUrl is required and must be a non-empty string.');
        }

        $token = null;
        if (isset($options['token']) && \is_string($options['token']) && $options['token'] !== '') {
            $token = $options['token'];
        }

        $allowInsecure = (bool) ($options['allowInsecureHttp'] ?? $options['allow_insecure_http'] ?? false);

        $connectTimeoutMs = (int) ($options['connectTimeoutMs'] ?? $options['connect_timeout_ms'] ?? 1_000);
        $readTimeoutMs = (int) ($options['readTimeoutMs'] ?? $options['read_timeout_ms'] ?? 5_000);
        $writeTimeoutMs = (int) ($options['writeTimeoutMs'] ?? $options['write_timeout_ms'] ?? 5_000);
        $overallTimeoutMs = (int) ($options['overallTimeoutMs'] ?? $options['overall_timeout_ms'] ?? 10_000);

        $maxRetries = (int) ($options['maxRetries'] ?? $options['max_retries'] ?? 0);
        $retryBackoffMs = (int) ($options['retryBackoffMs'] ?? $options['retry_backoff_ms'] ?? 100);

        $httpClient = $options['httpClient'] ?? null;
        if ($httpClient !== null && !$httpClient instanceof HttpClientInterface) {
            throw new RonConfigException('httpClient must implement HttpClientInterface.');
        }

        return new self(
            $baseUrl,
            $token,
            $allowInsecure,
            $connectTimeoutMs,
            $readTimeoutMs,
            $writeTimeoutMs,
            $overallTimeoutMs,
            $maxRetries,
            $retryBackoffMs,
            $httpClient
        );
    }

    private static function normalizeBaseUrl(string $baseUrl, bool $allowInsecure): string
    {
        $trimmed = rtrim(trim($baseUrl), '/');

        if ($trimmed === '') {
            throw new RonConfigException('Gateway base URL must not be empty.');
        }

        $parts = \parse_url($trimmed);
        if ($parts === false || !isset($parts['scheme']) || !isset($parts['host'])) {
            throw new RonConfigException('Gateway base URL must be a valid absolute URL.');
        }

        $scheme = strtolower($parts['scheme']);

        if ($scheme === 'http' && !$allowInsecure) {
            throw new RonConfigException(
                'Insecure HTTP is disabled by default. Set RON_SDK_INSECURE_HTTP=1 or allowInsecureHttp=true explicitly.'
            );
        }

        if ($scheme !== 'http' && $scheme !== 'https') {
            throw new RonConfigException('Gateway base URL must use http or https scheme.');
        }

        return $trimmed;
    }

    private static function validatePositive(string $name, int $value): int
    {
        if ($value <= 0) {
            throw new RonConfigException(sprintf('%s must be a positive integer (ms).', $name));
        }

        return $value;
    }

    public function getBaseUrl(): string
    {
        return $this->baseUrl;
    }

    public function getToken(): ?string
    {
        return $this->token;
    }

    public function isInsecureHttpAllowed(): bool
    {
        return $this->allowInsecureHttp;
    }

    public function getConnectTimeoutMs(): int
    {
        return $this->connectTimeoutMs;
    }

    public function getReadTimeoutMs(): int
    {
        return $this->readTimeoutMs;
    }

    public function getWriteTimeoutMs(): int
    {
        return $this->writeTimeoutMs;
    }

    public function getOverallTimeoutMs(): int
    {
        return $this->overallTimeoutMs;
    }

    public function getMaxRetries(): int
    {
        return $this->maxRetries;
    }

    public function getRetryBackoffMs(): int
    {
        return $this->retryBackoffMs;
    }

    public function getHttpClient(): ?HttpClientInterface
    {
        return $this->httpClient;
    }

    public function withHttpClient(HttpClientInterface $client): self
    {
        $clone = clone $this;
        $clone->httpClient = $client;

        return $clone;
    }
}
