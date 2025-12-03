package dev.roncore.sdk.examples.java;

import dev.roncore.sdk.AppResponse;
import dev.roncore.sdk.RonClient;
import dev.roncore.sdk.RonException;

import java.util.Map;

/**
 * RO:WHAT —
 *   Minimal Java CLI example for the RON-CORE JVM SDK.
 *
 * RO:WHY —
 *   Give Java developers a zero-friction way to:
 *     * configure via env vars, and
 *     * hit a simple /app/ping endpoint on a Micronode or Macronode.
 *
 * RO:INTERACTS —
 *   - RonClient (core HTTP client)
 *   - EnvConfigLoader (via RonClient.Builder.fromEnv())
 *   - JSON mapper (Jackson-based, via AppResponse<Map<String, Object>>)
 *
 * RO:INVARIANTS —
 *   - Requires RON_SDK_GATEWAY_ADDR to be set (e.g. "http://127.0.0.1:5304").
 *   - Uses /ping path which the SDK normalizes to /app/ping.
 *   - Respects RON_SDK_INSECURE_HTTP for local HTTP development.
 */
public final class HelloCli {

    private HelloCli() {
        // static-only
    }

    public static void main(String[] args) {
        System.out.println("=== RON-CORE JVM SDK — Hello CLI ===");
        System.out.println();

        String baseUrl = System.getenv("RON_SDK_GATEWAY_ADDR");
        if (baseUrl == null || baseUrl.isBlank()) {
            System.err.println("ERROR: RON_SDK_GATEWAY_ADDR is not set.");
            System.err.println();
            System.err.println("Set it to your Micronode/Macronode gateway URL, for example:");
            System.err.println("  RON_SDK_GATEWAY_ADDR=http://127.0.0.1:5304");
            System.err.println("  RON_SDK_INSECURE_HTTP=1   # if using HTTP instead of HTTPS");
            System.exit(1);
        }

        System.out.println("Using RON_SDK_GATEWAY_ADDR = " + baseUrl);
        String insecureFlag = System.getenv("RON_SDK_INSECURE_HTTP");
        if (insecureFlag != null) {
            System.out.println("RON_SDK_INSECURE_HTTP       = " + insecureFlag);
        }
        System.out.println();

        RonClient client;
        try {
            // Builder.fromEnv() pulls all RON_SDK_* env vars, including gateway + timeouts.
            client = RonClient.builder()
                    .fromEnv()
                    .build();
        } catch (RonException ex) {
            System.err.println("Failed to build RonClient from env:");
            ex.printStackTrace(System.err);
            System.exit(2);
            return; // unreachable, but keeps compiler happy
        }

        try {
            // The SDK will normalize "/ping" → "/app/ping" against the configured base URL.
            AppResponse<Map> response = client.get("/ping", Map.class);

            System.out.println("HTTP status: " + response.getStatus());
            if (response.ok()) {
                System.out.println("--- OK envelope ---");
                Map<?, ?> body = response.getData();
                if (body != null) {
                    System.out.println(body);
                } else {
                    System.out.println("(no data payload)");
                }
            } else {
                System.out.println("--- Problem envelope ---");
                System.out.println(response.getProblem());
            }
        } catch (RonException ex) {
            System.err.println("RON-CORE call failed with RonException:");
            ex.printStackTrace(System.err);
            System.exit(3);
        } catch (Exception ex) {
            System.err.println("Unexpected error while calling RON-CORE:");
            ex.printStackTrace(System.err);
            System.exit(4);
        }
    }
}
