<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp;

/**
 * RO:WHAT — Canonical error envelope value object.
 * RO:WHY  — Wraps RON-CORE canonical error schema + RFC7807 variant.
 * RO:INTERACTS — RonProblemException, RonClient error mapping, logs.
 * RO:INVARIANTS —
 *   * Safe to log; never contains raw secrets.
 *   * Handles both RON canonical fields and RFC7807 fields.
 */
final class Problem
{
    private ?string $code;
    private ?string $message;
    private ?string $kind;
    private ?string $correlationId;
    private mixed $details;

    // RFC 7807-ish fields
    private ?string $type;
    private ?string $title;
    private ?int $status;
    private ?string $detail;
    private ?string $instance;

    /**
     * Arbitrary extension members.
     *
     * @var array<string,mixed>
     */
    private array $extensions;

    private ?string $canonicalMessage;

    /**
     * @param array<string,mixed> $extensions
     */
    private function __construct(
        ?string $code,
        ?string $message,
        ?string $kind,
        ?string $correlationId,
        mixed $details,
        ?string $type,
        ?string $title,
        ?int $status,
        ?string $detail,
        ?string $instance,
        array $extensions,
        ?string $canonicalMessage = null
    ) {
        $this->code = $code;
        $this->message = $message;
        $this->kind = $kind;
        $this->correlationId = $correlationId;
        $this->details = $details;
        $this->type = $type;
        $this->title = $title;
        $this->status = $status;
        $this->detail = $detail;
        $this->instance = $instance;
        $this->extensions = $extensions;
        $this->canonicalMessage = $canonicalMessage;
    }

    /**
     * Build a Problem from canonical / RFC7807 JSON.
     *
     * @param array<string,mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $code = isset($data['code']) && \is_string($data['code']) ? $data['code'] : null;
        $message = isset($data['message']) && \is_string($data['message']) ? $data['message'] : null;
        $kind = isset($data['kind']) && \is_string($data['kind']) ? $data['kind'] : null;

        $correlationId = null;
        if (isset($data['correlation_id']) && \is_string($data['correlation_id'])) {
            $correlationId = $data['correlation_id'];
        } elseif (isset($data['correlationId']) && \is_string($data['correlationId'])) {
            $correlationId = $data['correlationId'];
        }

        $details = $data['details'] ?? null;

        $type = isset($data['type']) && \is_string($data['type']) ? $data['type'] : null;
        $title = isset($data['title']) && \is_string($data['title']) ? $data['title'] : null;

        $status = null;
        if (isset($data['status'])) {
            if (\is_int($data['status'])) {
                $status = $data['status'];
            } elseif (\is_string($data['status']) && ctype_digit($data['status'])) {
                $status = (int) $data['status'];
            }
        }

        $detail = isset($data['detail']) && \is_string($data['detail']) ? $data['detail'] : null;
        $instance = isset($data['instance']) && \is_string($data['instance']) ? $data['instance'] : null;

        // Everything that is not a canonical / RFC7807 field is an extension.
        $reservedKeys = [
            'code',
            'message',
            'kind',
            'correlation_id',
            'correlationId',
            'details',
            'type',
            'title',
            'status',
            'detail',
            'instance',
        ];

        $extensions = [];

        foreach ($data as $key => $value) {
            if (\in_array($key, $reservedKeys, true)) {
                continue;
            }

            if (\is_string($key)) {
                $extensions[$key] = $value;
            }
        }

        $canonicalMessage = $message;

        return new self(
            $code,
            $message,
            $kind,
            $correlationId,
            $details,
            $type,
            $title,
            $status,
            $detail,
            $instance,
            $extensions,
            $canonicalMessage
        );
    }

    /**
     * @return array<string,mixed>
     */
    public function toArray(): array
    {
        $base = [
            'code'           => $this->code,
            'message'        => $this->message,
            'kind'           => $this->kind,
            'correlation_id' => $this->correlationId,
            'details'        => $this->details,
            'type'           => $this->type,
            'title'          => $this->title,
            'status'         => $this->status,
            'detail'         => $this->detail,
            'instance'       => $this->instance,
        ];

        // Remove nulls for a cleaner payload.
        $base = array_filter(
            $base,
            static fn ($value): bool => $value !== null
        );

        return array_merge($base, $this->extensions);
    }

    public function getCode(): ?string
    {
        return $this->code;
    }

    public function getMessage(): ?string
    {
        return $this->message;
    }

    public function getKind(): ?string
    {
        return $this->kind;
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

    public function getType(): ?string
    {
        return $this->type;
    }

    public function getTitle(): ?string
    {
        return $this->title;
    }

    public function getStatus(): ?int
    {
        return $this->status;
    }

    public function getDetail(): ?string
    {
        return $this->detail;
    }

    public function getInstance(): ?string
    {
        return $this->instance;
    }

    /**
     * @return array<string,mixed>
     */
    public function getExtensions(): array
    {
        return $this->extensions;
    }

    /**
     * Prefer canonical message, then title, then detail.
     */
    public function getCanonicalMessage(): string
    {
        if ($this->canonicalMessage !== null && $this->canonicalMessage !== '') {
            return $this->canonicalMessage;
        }

        if ($this->title !== null && $this->title !== '') {
            return $this->title;
        }

        if ($this->detail !== null && $this->detail !== '') {
            return $this->detail;
        }

        if ($this->code !== null && $this->code !== '') {
            return $this->code;
        }

        return 'Unknown error';
    }
}
