<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Util;

/**
 * RO:WHAT — Helper for generating/validating idempotency keys.
 * RO:WHY — Supports safe retries for write operations.
 * RO:INTERACTS — IdempotencyMiddleware, RonClient (future).
 * RO:INVARIANTS — Keys are opaque, URL-safe, and bounded in length.
 */
final class IdempotencyKey
{
    private const MAX_LENGTH = 128;

    private string $value;

    private function __construct(string $value)
    {
        $trimmed = trim($value);

        if ($trimmed === '') {
            throw new \InvalidArgumentException('Idempotency key must not be empty.');
        }

        if (\strlen($trimmed) > self::MAX_LENGTH) {
            throw new \InvalidArgumentException('Idempotency key is too long.');
        }

        $this->value = $trimmed;
    }

    public static function generate(): self
    {
        // 16 bytes -> 32 hex characters; short, URL-safe, and unique enough
        $bytes = \random_bytes(16);

        return new self(\bin2hex($bytes));
    }

    public static function fromString(string $value): self
    {
        return new self($value);
    }

    public function asString(): string
    {
        return $this->value;
    }

    public function __toString(): string
    {
        return $this->value;
    }
}
