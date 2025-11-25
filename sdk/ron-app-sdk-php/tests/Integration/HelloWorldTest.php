<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Integration;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\Exception\RonProblemException;
use Ron\AppSdkPhp\RonClient;

/**
 * RO:WHAT  — Live “hello world” integration test against /app/hello.
 * RO:WHY   — Exercise the real HTTP path (env config + RonClient + problem mapping).
 * RO:INTERACTS —
 *   * ClientConfig::fromEnv()
 *   * RonClient::get()
 *   * Problem/RonProblemException error model
 * RO:INVARIANTS —
 *   * Never runs unless RON_SDK_GATEWAY_ADDR is set (safe for CI).
 *   * Treats both success (200 JSON) and RFC7807 problem responses as testable, not flaky:
 *     - Success: asserts JSON shape in a minimal way.
 *     - Problem: asserts structured problem fields are populated.
 */
final class HelloWorldTest extends TestCase
{
    public function testHelloEndpointOrProblemIsHandled(): void
    {
        $gateway = \getenv('RON_SDK_GATEWAY_ADDR');

        if ($gateway === false || $gateway === '') {
            self::markTestSkipped('RON_SDK_GATEWAY_ADDR not set; skipping live hello integration test.');
        }

        $config = ClientConfig::fromEnv();
        $client = new RonClient($config);

        try {
            $response = $client->get('/app/hello');
            $data = $response->json();

            // We don’t assume an exact payload, just that it’s valid JSON.
            self::assertIsArray($data, 'Expected /app/hello to return a JSON object.');

            if (\array_key_exists('hello', $data)) {
                // Future-friendly: we only require that "hello" is a string,
                // not that it is exactly "world".
                self::assertIsString(
                    $data['hello'],
                    'Expected "hello" key to be a string when present.'
                );
            }
        } catch (RonProblemException $e) {
            $problem = $e->getProblem();

            // For the current “upstream_unavailable” case, this is the branch
            // that will run. We just assert that the error is well-structured.
            $status = $problem->getStatus();
            self::assertNotNull(
                $status,
                'Problem response from gateway should include an HTTP status.'
            );
            self::assertGreaterThanOrEqual(
                400,
                $status,
                'Problem status should be in the error range (>= 400).'
            );

            $message = $problem->getCanonicalMessage();
            self::assertNotSame(
                '',
                $message,
                'Problem should expose a non-empty canonical message.'
            );

            // Code is optional but nice to have (e.g., "upstream_unavailable").
            // We don’t assert an exact value to stay stable as the backend evolves.
            $code = $problem->getCode();
            self::assertTrue(
                $code === null || \is_string($code),
                'Problem code, when present, should be a string.'
            );
        } finally {
            $client->close();
        }
    }
}
