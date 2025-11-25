#!/usr/bin/env php
<?php

declare(strict_types=1);

/*
 * RO:WHAT  — Pagination example using Paginator and a hypothetical /app/resources endpoint.
 * RO:WHY   — Demonstrates how to iterate across all pages without manual page_token plumbing.
 * RO:INTERACTS — RonClient::get(), Page, Paginator.
 * RO:INVARIANTS —
 *   * Assumes the API returns: { "items": [...], "next_page_token": "..." }.
 *   * Uses page_token query parameter, matching SDK_SCHEMA_IDB.
 */

require __DIR__ . '/../vendor/autoload.php';

use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\Exception\RonAuthException;
use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonProblemException;
use Ron\AppSdkPhp\Page;
use Ron\AppSdkPhp\Paginator;
use Ron\AppSdkPhp\RonClient;

function stderr(string $msg): void
{
    fwrite(STDERR, $msg . PHP_EOL);
}

try {
    $config = ClientConfig::fromEnv();
    $client = new RonClient($config);

    /**
     * @param string|null $pageToken
     */
    $fetchPage = function (?string $pageToken) use ($client): Page {
        $query = [];

        if ($pageToken !== null) {
            $query['page_token'] = $pageToken;
        }

        // Example endpoint; actual path may vary by app:
        // GET /app/resources?page_token=...
        $response = $client->get('/app/resources', $query);
        $data = $response->json();

        if (!is_array($data)) {
            throw new \RuntimeException('Expected JSON object from /app/resources.');
        }

        $items = [];
        if (isset($data['items']) && is_array($data['items'])) {
            $items = $data['items'];
        }

        $nextToken = null;
        if (isset($data['next_page_token']) && is_string($data['next_page_token'])) {
            $nextToken = $data['next_page_token'];
        }

        return new Page($items, $nextToken);
    };

    echo "Streaming items from /app/resources...\n";

    $count = 0;

    foreach (Paginator::iterate($fetchPage) as $item) {
        // For demo purposes, just print each item as JSON on its own line.
        echo json_encode($item, JSON_UNESCAPED_SLASHES) . PHP_EOL;
        ++$count;
    }

    echo sprintf("Done. Total items: %d\n", $count);

    $client->close();
    exit(0);
} catch (RonProblemException $e) {
    $problem = $e->getProblem();
    $code = $problem->getCode() ?? 'unknown';
    $msg = $problem->getCanonicalMessage();

    stderr(sprintf('[RON] Problem (%s): %s', $code, $msg));
    exit(2);
} catch (RonAuthException $e) {
    stderr('[RON] Auth error: ' . $e->getMessage());
    exit(3);
} catch (RonNetworkException $e) {
    stderr('[RON] Network error: ' . $e->getMessage());
    exit(4);
} catch (\Throwable $e) {
    stderr('[RON] Unexpected error running pagination example: ' . $e->getMessage());
    exit(1);
}
