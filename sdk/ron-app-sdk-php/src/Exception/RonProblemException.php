<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Exception;

use Ron\AppSdkPhp\Problem;

/**
 * RO:WHAT — Exception for structured Problem responses from RON-CORE.
 * RO:WHY — Wraps canonical error envelope + RFC7807 in one value.
 * RO:INTERACTS — Problem DTO, RonClient error mapping.
 * RO:INVARIANTS — Canonical getters (code/kind/message/correlationId/details).
 */
final class RonProblemException extends RonException
{
    private Problem $problem;

    public function __construct(Problem $problem, ?\Throwable $previous = null)
    {
        $message = $problem->getCanonicalMessage() ?? 'Request failed with a problem response.';

        parent::__construct(
            $message,
            $problem->getCode(),
            $problem->getStatus(),
            $problem->getCorrelationId(),
            $problem->getDetails(),
            $previous
        );

        $this->problem = $problem;
    }

    public function getProblem(): Problem
    {
        return $this->problem;
    }

    /**
     * Canonical error code (alias for Problem::getCode()).
     */
    public function getErrorCode(): ?string
    {
        return $this->problem->getCode();
    }

    public function getKind(): ?string
    {
        return $this->problem->getKind();
    }

    public function getCanonicalMessage(): ?string
    {
        return $this->problem->getCanonicalMessage();
    }
}
