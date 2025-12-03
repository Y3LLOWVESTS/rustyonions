package dev.roncore.sdk;

import java.util.Collections;
import java.util.HashMap;
import java.util.Map;

/**
 * RO:WHAT —
 *   JVM exception type for SDK-level failures (config, transport, policy, app).
 *
 * RO:WHY —
 *   Gives callers a single, structured exception to catch, without leaking
 *   tokens or sensitive internals into messages or stack traces.
 *
 * RO:INTERACTS —
 *   - Thrown by {@link RonClient} for configuration and transport failures.
 *   - May wrap a {@link RonProblem} for app-level problems where throwing
 *     is more ergonomic than returning {@link AppResponse}.
 *
 * RO:INVARIANTS —
 *   - No secret material (tokens, headers) is ever embedded in message text.
 *   - {@code kind} and {@code code} are stable, machine-readable identifiers.
 */
public final class RonException extends RuntimeException {

    private final String kind;
    private final String code;
    private final String correlationId;
    private final boolean retryable;
    private final Map<String, Object> details;

    public RonException(
            String kind,
            String code,
            String message,
            String correlationId,
            boolean retryable,
            Map<String, Object> details,
            Throwable cause
    ) {
        super(message, cause);
        this.kind = kind;
        this.code = code;
        this.correlationId = correlationId;
        this.retryable = retryable;
        this.details = details != null ? Collections.unmodifiableMap(new HashMap<>(details)) : Collections.emptyMap();
    }

    public String getKind() {
        return kind;
    }

    public String getCode() {
        return code;
    }

    public String getCorrelationId() {
        return correlationId;
    }

    public boolean isRetryable() {
        return retryable;
    }

    public Map<String, Object> getDetails() {
        return details;
    }

    public static RonException configError(String code, String message) {
        return new RonException("config", code, message, null, false, Collections.emptyMap(), null);
    }

    public static RonException transportError(String code, String message, boolean retryable, Throwable cause) {
        return new RonException("transport", code, message, null, retryable, Collections.emptyMap(), cause);
    }

    public static RonException decodeError(String message, Throwable cause) {
        return new RonException("transport", "DECODE_ERROR", message, null, false, Collections.emptyMap(), cause);
    }
}
