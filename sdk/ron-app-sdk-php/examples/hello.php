#!/usr/bin/env php
<?php

declare(strict_types=1);

/*
 * RO:WHAT  — Minimal “hello world” CLI example for ron-app-sdk-php.
 * RO:WHY   — Quick manual smoke test against /app/hello.
 * RO:INTERACTS — ClientConfig::fromEnv(), RonClient::get().
 * RO:INVARIANTS —
 *   * Reads configuration from env (RON_SDK_*).
 *   * Exits non-zero on error, but never logs secrets.
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

    // By convention, /app/hello is the simplest “is the app plane alive?” facet.
    $response = $client->get('/app/hello');
    $data = $response->json();

    echo json_encode($data, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES) . PHP_EOL;

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
    // Last-resort handler; message is generic to avoid leaking secrets.
    stderr('[RON] Unexpected error running hello example: ' . $e->getMessage());
    exit(1);
}
