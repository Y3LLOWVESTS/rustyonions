package dev.roncore.sdk.examples.spring;

import dev.roncore.sdk.AppResponse;
import dev.roncore.sdk.RonClient;
import dev.roncore.sdk.RonException;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;
import org.springframework.http.HttpStatus;
import org.springframework.http.ResponseEntity;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.Map;

/**
 * RO:WHAT —
 *   Simple controller that proxies a /ron/ping HTTP GET to RON-CORE.
 *
 * RO:WHY —
 *   Shows how to consume RonClient from a Spring REST controller
 *   and surface either OK data or a problem envelope.
 *
 * RO:INVARIANTS —
 *   - Uses RonClient configured via RonSdkConfig (RON_SDK_* env vars).
 *   - Returns 200 with data on success.
 *   - Returns 502 with a descriptive message on failure.
 */
@RestController
public class HelloController {

    private static final Logger log = LoggerFactory.getLogger(HelloController.class);

    private final RonClient client;

    public HelloController(RonClient client) {
        this.client = client;
    }

    @GetMapping("/ron/ping")
    public ResponseEntity<?> ping() {
        try {
            // The SDK normalizes "/ping" -> "/app/ping" on the gateway.
            AppResponse<Map> response = client.get("/ping", Map.class);

            log.info("RON-CORE /ping status={} ok={}", response.getStatus(), response.ok());

            if (response.ok()) {
                Map<?, ?> body = response.getData();
                return ResponseEntity.ok(body != null ? body : Map.of("note", "no data payload"));
            } else {
                // Bubble up the problem as a 502 for demo purposes.
                return ResponseEntity
                    .status(HttpStatus.BAD_GATEWAY)
                    .body(Map.of(
                        "error", "RON-CORE returned a problem envelope",
                        "status", response.getStatus(),
                        "problem", String.valueOf(response.getProblem())
                    ));
            }
        } catch (RonException ex) {
            log.warn("RON-CORE call failed with RonException", ex);
            return ResponseEntity
                .status(HttpStatus.BAD_GATEWAY)
                .body(Map.of(
                    "error", "RON-CORE call failed",
                    "type", ex.getClass().getSimpleName(),
                    "message", String.valueOf(ex.getMessage())
                ));
        } catch (Exception ex) {
            log.error("Unexpected error while calling RON-CORE", ex);
            return ResponseEntity
                .status(HttpStatus.INTERNAL_SERVER_ERROR)
                .body(Map.of(
                    "error", "Unexpected error",
                    "type", ex.getClass().getSimpleName(),
                    "message", String.valueOf(ex.getMessage())
                ));
        }
    }
}
