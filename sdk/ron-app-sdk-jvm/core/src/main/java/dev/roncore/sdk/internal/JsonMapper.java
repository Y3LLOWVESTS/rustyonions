package dev.roncore.sdk.internal;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import dev.roncore.sdk.AppResponse;
import dev.roncore.sdk.RonException;
import dev.roncore.sdk.RonProblem;
import dev.roncore.sdk.http.HttpResponse;

/**
 * RO:WHAT —
 *   Thin wrapper around Jackson for decoding RON envelopes.
 *
 * RO:WHY —
 *   Centralizes JSON mapping decisions (unknown fields, extra map, etc.)
 *   so they are consistent across the SDK.
 */
public final class JsonMapper {

    private final ObjectMapper mapper;

    public JsonMapper(ObjectMapper mapper) {
        this.mapper = mapper;
    }

    public JsonMapper() {
        this(new ObjectMapper());
    }

    public String toJson(Object value) {
        if (value == null) {
            return null;
        }
        try {
            return mapper.writeValueAsString(value);
        } catch (JsonProcessingException e) {
            throw RonException.decodeError("Failed to serialize request body", e);
        }
    }

    public <T> AppResponse<T> decodeAppResponse(HttpResponse httpResponse, Class<T> dataType) {
        int status = httpResponse.getStatusCode();
        String body = httpResponse.getBody();

        if (body == null || body.isBlank()) {
            return new AppResponse<>(null, null, status);
        }

        try {
            JsonNode root = mapper.readTree(body);

            RonProblem problem = null;
            if (root.hasNonNull("problem")) {
                problem = mapper.treeToValue(root.get("problem"), RonProblem.class);
            }

            T data = null;
            if (root.hasNonNull("data") && dataType != null && !dataType.equals(Void.class)) {
                JsonNode dataNode = root.get("data");
                data = mapper.treeToValue(dataNode, dataType);
            }

            return new AppResponse<>(data, problem, status);
        } catch (Exception e) {
            throw RonException.decodeError("Failed to decode response body", e);
        }
    }
}
