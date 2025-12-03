package dev.roncore.sdk;

import com.fasterxml.jackson.annotation.JsonAnySetter;
import com.fasterxml.jackson.annotation.JsonIgnore;
import com.fasterxml.jackson.annotation.JsonProperty;
import java.util.Collections;
import java.util.HashMap;
import java.util.Map;
import java.util.Objects;

/**
 * RO:WHAT —
 *   JVM representation of the canonical RON problem envelope.
 *
 * RO:WHY —
 *   Gives callers a structured view of application, policy, and transport
 *   problems instead of ad-hoc strings.
 *
 * RO:INTERACTS —
 *   - {@link AppResponse} (references a {@code RonProblem} when not ok()).
 *   - {@link RonException} (may wrap a {@code RonProblem}).
 *
 * RO:INVARIANTS —
 *   - Fields mirror the cross-language schema: code, message, kind,
 *     correlation_id, details.
 *   - Unknown fields are captured into {@code extra} but never crash parsing.
 */
public final class RonProblem {

    @JsonProperty("code")
    private final String code;

    @JsonProperty("message")
    private final String message;

    @JsonProperty("kind")
    private final String kind;

    @JsonProperty("correlation_id")
    private final String correlationId;

    @JsonProperty("details")
    private final Map<String, Object> details;

    @JsonIgnore
    private final Map<String, Object> extra;

    public RonProblem(
            @JsonProperty("code") String code,
            @JsonProperty("message") String message,
            @JsonProperty("kind") String kind,
            @JsonProperty("correlation_id") String correlationId,
            @JsonProperty("details") Map<String, Object> details
    ) {
        this.code = code;
        this.message = message;
        this.kind = kind;
        this.correlationId = correlationId;
        this.details = details != null ? Collections.unmodifiableMap(new HashMap<>(details)) : Collections.emptyMap();
        this.extra = new HashMap<>();
    }

    @JsonAnySetter
    private void putExtra(String key, Object value) {
        extra.put(key, value);
    }

    public String getCode() {
        return code;
    }

    public String getMessage() {
        return message;
    }

    public String getKind() {
        return kind;
    }

    public String getCorrelationId() {
        return correlationId;
    }

    public Map<String, Object> getDetails() {
        return details;
    }

    /**
     * Any additional fields the server may have sent that are not part
     * of the canonical envelope. Never null.
     */
    public Map<String, Object> getExtra() {
        return Collections.unmodifiableMap(extra);
    }

    @Override
    public String toString() {
        return "RonProblem{" +
                "code='" + code + '\'' +
                ", kind='" + kind + '\'' +
                ", correlationId='" + correlationId + '\'' +
                '}';
    }

    @Override
    public boolean equals(Object obj) {
        if (!(obj instanceof RonProblem other)) {
            return false;
        }
        return Objects.equals(code, other.code)
                && Objects.equals(message, other.message)
                && Objects.equals(kind, other.kind)
                && Objects.equals(correlationId, other.correlationId)
                && Objects.equals(details, other.details)
                && Objects.equals(extra, other.extra);
    }

    @Override
    public int hashCode() {
        return Objects.hash(code, message, kind, correlationId, details, extra);
    }
}
