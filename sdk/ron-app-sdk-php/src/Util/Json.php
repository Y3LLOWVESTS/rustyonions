<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Util;

/**
 * RO:WHAT — JSON encode/decode helpers with safe error messages.
 * RO:WHY — Avoids leaking raw payloads while surfacing structured errors.
 * RO:INTERACTS — Response::json(), RonClient request bodies.
 * RO:INVARIANTS — Uses JSON_THROW_ON_ERROR under the hood; wraps errors.
 */
final class Json
{
    private function __construct()
    {
    }

    /**
     * @return mixed
     */
    public static function decode(string $json, bool $assoc = true): mixed
    {
        try {
            return \json_decode($json, $assoc, 512, \JSON_THROW_ON_ERROR);
        } catch (\JsonException $e) {
            // Intentionally generic; do not include the raw JSON in the message.
            throw new \RuntimeException('Failed to decode JSON response from RON-CORE.', 0, $e);
        }
    }

    public static function encode(mixed $value): string
    {
        try {
            return \json_encode($value, \JSON_THROW_ON_ERROR);
        } catch (\JsonException $e) {
            // Intentionally generic; caller should log context, not payload.
            throw new \RuntimeException('Failed to encode request body as JSON.', 0, $e);
        }
    }
}
