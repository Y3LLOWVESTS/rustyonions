<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Exception;

/**
 * RO:WHAT — Configuration/validation error for ron-app-sdk-php.
 * RO:WHY — Fail-closed when env/config is invalid or incomplete.
 * RO:INTERACTS — ClientConfig::fromEnv(), ClientConfig::fromArray().
 * RO:INVARIANTS — Never falls back to unsafe defaults on invalid config.
 */
final class RonConfigException extends RonException
{
}
