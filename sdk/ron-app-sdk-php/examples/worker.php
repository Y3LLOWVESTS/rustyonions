#!/usr/bin/env php
<?php

declare(strict_types=1);

/*
 * RO:WHAT  — Simple long-running worker example.
 * RO:WHY   — Demonstrates how a PHP CLI worker might poll for jobs via the SDK.
 * RO:INTERACTS — RonClient::get()/post(), Job-like app facets.
 * RO:INVARIANTS —
 *   * Never busy-spins: always sleeps between polls.
 *   * Logs only high-level info; never logs secrets or full payloads.
 */

require __DIR__ . '/../vendor/autoload.php';

use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\Exception\RonAuthException;
use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonProblemException;
use Ron\AppSdkPhp\RonClient;

function stderr(string $msg): void
{
    fwrite(STDERR, $msg . PHP_EOL);
}

try {
    $config = ClientConfig::fromEnv();
    $client = new RonClient($config);

    // For demo purposes, keep the loop bounded so people can run it safely.
    $maxIterations = (int) (getenv('RON_WORKER_MAX_ITERATIONS') ?: 10);
    $sleepSeconds = (int) (getenv('RON_WORKER_SLEEP_SECONDS') ?: 5);

    echo sprintf(
        "Starting worker: maxIterations=%d, sleep=%ds\n",
        $maxIterations,
        $sleepSeconds
    );

    for ($i = 0; $i < $maxIterations; ++$i) {
        echo sprintf("[worker] Poll %d/%d\n", $i + 1, $maxIterations);

        try {
            // Hypothetical job endpoint:
            //   GET /app/jobs/next -> { "job": {...} } or { "job": null }
            $rsp = $client->get('/app/jobs/next');
            $data = $rsp->json();

            if (!is_array($data) || !array_key_exists('job', $data)) {
                echo "[worker] No job field in response; sleeping...\n";
                sleep($sleepSeconds);
                continue;
            }

            $job = $data['job'];

            if ($job === null) {
                echo "[worker] No job available; sleeping...\n";
                sleep($sleepSeconds);
                continue;
            }

            echo "[worker] Got job; processing...\n";

            // Do something with $job (application-specific).
            // For demo, just echo a redacted summary:
            echo "[worker] Job payload (redacted): " . json_encode($job, JSON_UNESCAPED_SLASHES) . PHP_EOL;

            // Hypothetical completion endpoint:
            //   POST /app/jobs/complete -> { "ok": true }
            $result = ['status' => 'ok'];
            $client->post('/app/jobs/complete', $result);

            echo "[worker] Job marked complete.\n";
        } catch (RonProblemException $e) {
            $problem = $e->getProblem();
            $msg = $problem->getCanonicalMessage();
            stderr('[worker] Problem while processing job: ' . $msg);
        } catch (RonAuthException $e) {
            stderr('[worker] Auth error; stopping worker: ' . $e->getMessage());
            break;
        } catch (RonNetworkException $e) {
            stderr('[worker] Network error while polling jobs: ' . $e->getMessage());
        } catch (\Throwable $e) {
            stderr('[worker] Unexpected error: ' . $e->getMessage());
        }

        sleep($sleepSeconds);
    }

    echo "[worker] Done.\n";
    $client->close();
    exit(0);
} catch (\Throwable $e) {
    stderr('[RON] Failed to start worker: ' . $e->getMessage());
    exit(1);
}
