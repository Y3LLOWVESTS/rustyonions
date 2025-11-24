<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Exception;

/**
 * RO:WHAT — Network/transport-level error (DNS, connect failure, etc).
 * RO:WHY — Separates underlying transport faults from Problem errors.
 * RO:INTERACTS — HTTP client adapters (Guzzle/Symfony), RonClient.
 * RO:INVARIANTS — Messages are generic, no raw response bodies.
 */
final class RonNetworkException extends RonException
{
}
