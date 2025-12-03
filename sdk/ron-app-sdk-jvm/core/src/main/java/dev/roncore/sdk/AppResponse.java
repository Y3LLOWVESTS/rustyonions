package dev.roncore.sdk;

import java.util.Objects;

/**
 * RO:WHAT —
 *   Canonical application-plane response envelope for the JVM SDK.
 *
 * RO:WHY —
 *   Wraps either successful data or a {@link RonProblem} plus HTTP status,
 *   so call sites can handle success/error in a uniform way.
 *
 * RO:INTERACTS —
 *   - {@link RonClient} (returns this from HTTP calls).
 *   - {@link RonProblem} (structured error details).
 *
 * RO:INVARIANTS —
 *   - For any HTTP response, exactly one of {@code data} or {@code problem}
 *     will be non-null in well-formed responses.
 *   - {@code status} is always the HTTP status code observed on the wire.
 */
public final class AppResponse<T> {

    private final T data;
    private final RonProblem problem;
    private final int status;

    public AppResponse(T data, RonProblem problem, int status) {
        this.data = data;
        this.problem = problem;
        this.status = status;
    }

    /**
     * Convenience factory for a successful response.
     */
    public static <T> AppResponse<T> ok(T data, int status) {
        return new AppResponse<>(data, null, status);
    }

    /**
     * Convenience factory for an error response.
     */
    public static <T> AppResponse<T> error(RonProblem problem, int status) {
        return new AppResponse<>(null, problem, status);
    }

    public T getData() {
        return data;
    }

    public RonProblem getProblem() {
        return problem;
    }

    public int getStatus() {
        return status;
    }

    /**
     * Returns true if the response is considered a success (2xx HTTP status and no problem).
     * This matches the shape expected by tests (ok()) while preserving the older isOk() name.
     */
    public boolean ok() {
        return status >= 200 && status < 300 && problem == null;
    }

    /**
     * Alias kept for earlier code that used isOk().
     */
    public boolean isOk() {
        return ok();
    }

    @Override
    public String toString() {
        return "AppResponse{" +
                "status=" + status +
                ", ok=" + ok() +
                ", data=" + (data != null ? data.toString() : "null") +
                ", problem=" + (problem != null ? problem.toString() : "null") +
                '}';
    }

    @Override
    public boolean equals(Object obj) {
        if (!(obj instanceof AppResponse<?> other)) {
            return false;
        }
        return status == other.status
                && Objects.equals(data, other.data)
                && Objects.equals(problem, other.problem);
    }

    @Override
    public int hashCode() {
        return Objects.hash(data, problem, status);
    }
}
