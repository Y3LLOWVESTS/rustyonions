<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp;

/**
 * RO:WHAT — Structured error/Problem DTO (canonical + RFC7807).
 * RO:WHY — Provides a single representation for all RON-CORE error envelopes.
 * RO:INTERACTS — RonProblemException, RonClient error mapping.
 * RO:INVARIANTS — Safe fields only; may be logged, but no secrets.
 */
final class Problem
{
    // Canonical envelope fields
    private ?string $code;
    private ?string $kind;
    private ?string $canonicalMessage;
    private ?string $correlationId;
    /** @var mixed */
    private mixed $details;

    // RFC7807 fields
    private ?string $type;
    private ?string $title;
    private ?int $status;
    private ?string $detail;
    private ?string $instance;

    /**
     * Extensions/extra fields not part of the canonical/RFC7807 core.
     *
     * @var array<string,mixed>
     */
    private array $extensions;

    /**
     * @param array<string,mixed> $data
     */
    public static function fromArray(array $data): self
    {
        $knownKeys = [
            'code',
            'kind',
            'message',
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
            if (!\in_array($key, $knownKeys, true)) {
                $extensions[$key] = $value;
            }
        }

        $code = isset($data['code']) && is_string($data['code']) ? $data['code'] : null;
        $kind = isset($data['kind']) && is_string($data['kind']) ? $data['kind'] : null;

        $canonicalMessage = null;
        if (isset($data['message']) && is_string($data['message'])) {
            $canonicalMessage = $data['message'];
        }

        $correlationId = null;
        if (isset($data['correlation_id']) && is_string($data['correlation_id'])) {
            $correlationId = $data['correlation_id'];
        } elseif (isset($data['correlationId']) && is_string($data['correlationId'])) {
            $correlationId = $data['correlationId'];
        }

        $details = $data['details'] ?? null;

        $type = isset($data['type']) && is_string($data['type']) ? $data['type'] : null;
        $title = isset($data['title']) && is_string($data['title']) ? $data['title'] : null;

        $status = null;
        if (isset($data['status'])) {
            if (\is_int($data['status'])) {
                $status = $data['status'];
            } elseif (\is_string($data['status']) && ctype_digit($data['status'])) {
                $status = (int) $data['status'];
            }
        }

        $detail = isset($data['detail']) && is_string($data['detail']) ? $data['detail'] : null;
        $instance = isset($data['instance']) && is_string($data['instance']) ? $data['instance'] : null;

        return new self(
            $code,
            $kind,
            $canonicalMessage,
            $correlationId,
            $details,
            $type,
            $title,
            $status,
            $detail,
            $instance,
            $extensions
        );
    }

    /**
     * @param array<string,mixed> $extensions
     */
    private function __construct(
        ?string $code,
        ?string $kind,
        ?string $canonicalMessage,
        ?string $correlationId,
        mixed $details,
        ?string $type,
        ?string $title,
        ?int $status,
        ?string $detail,
        ?string $instance,
        array $extensions
    ) {
        $this->code = $code;
        $this->kind = $kind;
        $this->canonicalMessage = $canonicalMessage;
        $this->correlationId = $correlationId;
        $this->details = $details;
        $this->type = $type;
        $this->title = $title;
        $this->status = $status;
        $this->detail = $detail;
        $this->instance = $instance;
        $this->extensions = $extensions;
    }

    public function getCode(): ?string
    {
        return $this->code;
    }

    public function getKind(): ?string
    {
        return $this->kind;
    }

    public function getCanonicalMessage(): ?string
    {
        if ($this->canonicalMessage !== null) {
            return $this->canonicalMessage;
        }

        if ($this->title !== null) {
            return $this->title;
        }

        if ($this->detail !== null) {
            return $this->detail;
        }

        return null;
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
     * @return array<string,mixed>
     */
    public function toArray(): array
    {
        $base = [
            'code'            => $this->code,
            'kind'            => $this->kind,
            'message'         => $this->canonicalMessage,
            'correlation_id'  => $this->correlationId,
            'details'         => $this->details,
            'type'            => $this->type,
            'title'           => $this->title,
            'status'          => $this->status,
            'detail'          => $this->detail,
            'instance'        => $this->instance,
        ];

        return array_merge($base, $this->extensions);
    }
}
