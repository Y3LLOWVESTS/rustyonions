<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Util;

use Ron\AppSdkPhp\Exception\RonConfigException;

/**
 * RO:WHAT — Small helper for reading typed env vars.
 * RO:WHY — Centralizes env parsing/validation for ClientConfig.
 * RO:INTERACTS — ClientConfig::fromEnv().
 * RO:INVARIANTS — Fail-closed on invalid types; no secret logging.
 */
final class Env
{
    private function __construct()
    {
    }

    public static function getString(string $name, ?string $default = null): ?string
    {
        $value = \getenv($name);

        if ($value === false || $value === '') {
            return $default;
        }

        return (string) $value;
    }

    public static function getInt(string $name, ?int $default = null): ?int
    {
        $value = \getenv($name);

        if ($value === false || $value === '') {
            return $default;
        }

        if (!\is_numeric($value)) {
            throw new RonConfigException(
                sprintf('Environment variable %s must be an integer, got "%s".', $name, $value)
            );
        }

        $int = (int) $value;

        if ($int < 0) {
            throw new RonConfigException(
                sprintf('Environment variable %s must not be negative, got "%d".', $name, $int)
            );
        }

        return $int;
    }

    public static function getBool(string $name, ?bool $default = null): ?bool
    {
        $value = \getenv($name);

        if ($value === false || $value === '') {
            return $default;
        }

        $normalized = strtolower(trim((string) $value));

        if (in_array($normalized, ['1', 'true', 'yes', 'on'], true)) {
            return true;
        }

        if (in_array($normalized, ['0', 'false', 'no', 'off'], true)) {
            return false;
        }

        throw new RonConfigException(
            sprintf('Environment variable %s must be a boolean-like value, got "%s".', $name, $value)
        );
    }
}
