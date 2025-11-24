<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Exception;

/**
 * RO:WHAT — Authentication/authorization error (caps/macaroons).
 * RO:WHY — Separates auth failures from generic network or Problem errors.
 * RO:INTERACTS — RonClient, gateway responses, Problem envelope.
 * RO:INVARIANTS — Never includes token/cap content in messages.
 */
final class RonAuthException extends RonException
{
}
