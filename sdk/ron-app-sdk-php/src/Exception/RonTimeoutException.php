<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Exception;

/**
 * RO:WHAT — Timeout error (connect/read/write/overall).
 * RO:WHY — Signals bounded network behavior in the SDK.
 * RO:INTERACTS — HTTP clients, retry middleware, RonClient.
 * RO:INVARIANTS — Clear, safe message; no raw body.
 */
final class RonTimeoutException extends RonException
{
}
