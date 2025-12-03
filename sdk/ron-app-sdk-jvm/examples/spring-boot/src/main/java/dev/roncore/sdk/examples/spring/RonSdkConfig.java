package dev.roncore.sdk.examples.spring;

import dev.roncore.sdk.RonClient;
import dev.roncore.sdk.RonException;
import org.springframework.context.annotation.Bean;
import org.springframework.context.annotation.Configuration;

/**
 * RO:WHAT —
 *   Spring Boot configuration that exposes a singleton RonClient bean.
 *
 * RO:WHY —
 *   Centralizes env-based config (RON_SDK_* vars) and makes it easy
 *   to inject RonClient into controllers and services.
 *
 * RO:INVARIANTS —
 *   - Reads RON_SDK_GATEWAY_ADDR and friends via EnvConfigLoader
 *     (RonClient.Builder.fromEnv()).
 *   - Fails fast on startup if the SDK config is invalid.
 */
@Configuration
public class RonSdkConfig {

    @Bean
    public RonClient ronClient() {
        try {
            return RonClient.builder()
                .fromEnv()
                .build();
        } catch (RonException ex) {
            throw new IllegalStateException("Failed to configure RonClient from env", ex);
        }
    }
}
