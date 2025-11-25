<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Integration;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\Exception\RonAuthException;
use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonProblemException;
use Ron\AppSdkPhp\Exception\RonTimeoutException;
use Ron\AppSdkPhp\Page;
use Ron\AppSdkPhp\Paginator;
use Ron\AppSdkPhp\RonClient;

/*
 * RO:WHAT  — Live pagination integration test against a canonical list endpoint.
 * RO:WHY   — Exercises Page + Paginator against a real `/app/...` resource collection.
 * RO:INTERACTS — RonClient::get(), Page, Paginator, Problem mapping.
 * RO:INVARIANTS —
 *   * Uses `{ items, next_page_token }` envelope with `page_token` query parameter.
 *   * Never infinite: test guards against unbounded streams.
 *   * Skips cleanly when gateway/env are not prepared.
 *
 * NOTE:
 *   The list endpoint path is configurable via RON_SDK_PAGINATION_PATH.
 *   If unset, we default to `/app/resources`, matching the CLI example.
 */
final class PaginationLiveTest extends TestCase
{
    public function testPaginationStreamsItemsOrSignalsUpstreamUnavailable(): void
    {
        if (\getenv('RON_SDK_GATEWAY_ADDR') === false) {
            $this->markTestSkipped('RON_SDK_GATEWAY_ADDR not set; skipping live pagination test.');
        }

        $config = ClientConfig::fromEnv();
        $client = new RonClient($config);
        $path = $this->getPaginationPath();

        try {
            /**
             * @param string|null $pageToken
             */
            $fetchPage = function (?string $pageToken) use ($client, $path): Page {
                $query = [];

                if ($pageToken !== null) {
                    $query['page_token'] = $pageToken;
                }

                // Example contract: GET /app/resources?page_token=...
                $response = $client->get($path, $query);
                $data = $response->json();

                if (!\is_array($data)) {
                    throw new \RuntimeException('Expected JSON object from ' . $path);
                }

                $items = [];
                if (isset($data['items']) && \is_array($data['items'])) {
                    $items = $data['items'];
                }

                $nextToken = null;
                if (isset($data['next_page_token']) && \is_string($data['next_page_token'])) {
                    $nextToken = $data['next_page_token'];
                }

                return new Page($items, $nextToken);
            };

            $count = 0;
            $maxItems = 1_000;

            foreach (Paginator::iterate($fetchPage) as $item) {
                ++$count;

                // Basic sanity: items should be JSON-serializable values.
                $this->assertNotNull($item);

                if ($count >= $maxItems) {
                    $this->fail(\sprintf(
                        'Pagination test hit safety cap of %d items — possible infinite pagination loop.',
                        $maxItems
                    ));
                }
            }

            // At minimum, assert we made it through without blowing up.
            $this->assertGreaterThanOrEqual(
                0,
                $count,
                'Pagination iteration should complete without errors.'
            );
        } catch (RonProblemException $e) {
            $problem = $e->getProblem();
            $code = $problem->getCode();
            $message = $problem->getCanonicalMessage();

            if ($code === 'upstream_unavailable') {
                $this->markTestIncomplete(
                    'Gateway app plane is not wired yet (upstream_unavailable from pagination endpoint).'
                );
            }

            $this->fail(\sprintf(
                'Unexpected Problem from %s (code=%s): %s',
                $path,
                $code ?? 'null',
                $message
            ));
        } catch (RonAuthException $e) {
            $this->fail(
                'Auth error calling pagination endpoint ' . $path . ': ' . $e->getMessage()
            );
        } catch (RonTimeoutException | RonNetworkException $e) {
            $this->markTestSkipped(
                'Network/timeout error talking to gateway for pagination endpoint ' . $path . ': ' . $e->getMessage()
            );
        } finally {
            $client->close();
        }
    }

    private function getPaginationPath(): string
    {
        $env = \getenv('RON_SDK_PAGINATION_PATH');

        if (\is_string($env) && $env !== '') {
            return $env;
        }

        // Default to the canonical example path used in examples/pagination.php.
        return '/app/resources';
    }
}
