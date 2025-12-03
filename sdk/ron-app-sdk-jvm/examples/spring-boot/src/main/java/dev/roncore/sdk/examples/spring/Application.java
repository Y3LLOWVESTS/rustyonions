package dev.roncore.sdk.examples.spring;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;

/**
 * RO:WHAT —
 *   Minimal Spring Boot application showcasing the RON-CORE JVM SDK.
 *
 * RO:WHY —
 *   Gives Spring / enterprise devs a copy-paste starting point for
 *   wiring RonClient into a standard Boot app.
 */
@SpringBootApplication
public class Application {

    public static void main(String[] args) {
        SpringApplication.run(Application.class, args);
    }
}
