<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Exception;

/**
 * RO:WHAT — Base exception type for ron-app-sdk-php.
 * RO:WHY — Provides a canonical shape for SDK errors (code/kind/correlation).
 * RO:INTERACTS — Problem value object, HTTP clients, RonClient.
 * RO:INVARIANTS — No secrets in messages; safe metadata only.
 */
abstract class RonException extends \RuntimeException
{
    /**
     * Canonical error code from the gateway/app plane (if available).
     */
    private ?string $errorCode;

    /**
     * HTTP status code associated with this error (if available).
     */
    private ?int $statusCode;

    /**
     * Correlation ID used by RON-CORE for tracing this request.
     */
    private ?string $correlationId;

    /**
     * Arbitrary structured details attached to this error (safe to log).
     *
     * @var mixed
     */
    private mixed $details;

    /**
     * @param string           $message       Human-readable, safe error message.
     * @param string|null      $errorCode     Canonical error code (if any).
     * @param int|null         $statusCode    HTTP status code (if any).
     * @param string|null      $correlationId Correlation ID for tracing.
     * @param mixed            $details       Optional structured details.
     * @param \Throwable|null  $previous      Previous exception for chaining.
     */
    public function __construct(
        string $message,
        ?string $errorCode = null,
        ?int $statusCode = null,
        ?string $correlationId = null,
        mixed $details = null,
        ?\Throwable $previous = null
    ) {
        // We intentionally ignore the numeric $code of \Exception and
        // use a separate canonical error code string instead.
        parent::__construct($message, 0, $previous);

        $this->errorCode = $errorCode;
        $this->statusCode = $statusCode;
        $this->correlationId = $correlationId;
        $this->details = $details;
    }

    public function getErrorCode(): ?string
    {
        return $this->errorCode;
    }

    public function getStatusCode(): ?int
    {
        return $this->statusCode;
    }

    public function getCorrelationId(): ?string
    {
        return $this->correlationId;
    }

    /**
     * @return mixed
     */
    public function getDetails(): mixed
    {
        return $this->details;
    }
}
