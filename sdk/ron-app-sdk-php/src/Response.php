<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp;

use Ron\AppSdkPhp\Util\Json;

/**
 * RO:WHAT — Immutable HTTP response wrapper used by RonClient.
 * RO:WHY  — Provides a stable shape for status/headers/body + JSON helper.
 * RO:INTERACTS — HttpClientInterface, RonClient, Problem mapping.
 * RO:INVARIANTS —
 *   * Headers are stored with lowercase keys.
 *   * Header values are arrays of strings (multi-value safe).
 *   * JSON decoding uses Json helper (never logs raw payloads).
 */
final class Response
{
    private int $statusCode;

    /**
     * Normalized headers: lowercase keys, array-of-string values.
     *
     * @var array<string,string[]>
     */
    private array $headers;

    private string $body;

    /**
     * @param array<string,string[]> $headers
     */
    public function __construct(int $statusCode, array $headers, string $body)
    {
        $this->statusCode = $statusCode;
        $this->headers = self::normalizeHeaders($headers);
        $this->body = $body;
    }

    public function getStatusCode(): int
    {
        return $this->statusCode;
    }

    /**
     * @return array<string,string[]>
     */
    public function getHeaders(): array
    {
        return $this->headers;
    }

    /**
     * @return string[]
     */
    public function getHeader(string $name): array
    {
        $key = strtolower($name);

        return $this->headers[$key] ?? [];
    }

    public function getBody(): string
    {
        return $this->body;
    }

    public function isSuccess(): bool
    {
        return $this->statusCode >= 200 && $this->statusCode < 300;
    }

    /**
     * Decode the response body as JSON.
     *
     * @param bool $assoc When true, return associative arrays (default).
     *
     * @return mixed
     */
    public function json(bool $assoc = true): mixed
    {
        return Json::decode($this->body, $assoc);
    }

    /**
     * @param array<string,mixed> $headers
     *
     * @return array<string,string[]>
     */
    private static function normalizeHeaders(array $headers): array
    {
        $normalized = [];

        foreach ($headers as $name => $value) {
            $key = strtolower((string) $name);

            if (\is_array($value)) {
                $values = array_values(array_map('strval', $value));
            } else {
                $values = [(string) $value];
            }

            $normalized[$key] = $values;
        }

        return $normalized;
    }
}
