<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Http;

use GuzzleHttp\Client;
use GuzzleHttp\ClientInterface;
use GuzzleHttp\Exception\ConnectException;
use GuzzleHttp\Exception\TransferException;
use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonTimeoutException;
use Ron\AppSdkPhp\Response;

/**
 * RO:WHAT — HttpClientInterface implementation backed by Guzzle.
 * RO:WHY  — Lets the SDK depend on a stable abstraction, not Guzzle directly.
 * RO:INTERACTS — ClientConfig, RonClient.
 * RO:INVARIANTS —
 *   * Never throws Guzzle exceptions directly; always maps to Ron* exceptions.
 *   * Never logs or exposes raw request/response bodies.
 */
final class GuzzleHttpClient implements HttpClientInterface
{
    private ClientInterface $client;

    public function __construct(?ClientInterface $client = null)
    {
        // http_errors=false → we always see the response, even on 4xx/5xx.
        $this->client = $client ?? new Client([
            'http_errors'     => false,
            'allow_redirects' => false,
        ]);
    }

    /**
     * @param array<string,string> $headers
     */
    public function request(
        string $method,
        string $url,
        array $headers = [],
        ?string $body = null,
        int $timeoutMs = 10_000
    ): Response {
        $timeoutSeconds = max(0.001, $timeoutMs / 1000.0);

        $options = [
            'headers'        => $headers,
            'body'           => $body,
            'timeout'        => $timeoutSeconds,
            'connect_timeout'=> $timeoutSeconds,
        ];

        try {
            $res = $this->client->request($method, $url, $options);
        } catch (ConnectException $e) {
            // Treat connection-level failures as network errors.
            throw new RonNetworkException(
                'Network error while connecting to RON-CORE gateway.',
                null,
                null,
                null,
                null,
                $e
            );
        } catch (TransferException $e) {
            // Some Guzzle adapters expose handler context with timeout info.
            $context = method_exists($e, 'getHandlerContext') ? $e->getHandlerContext() : null;

            if (\is_array($context) && ($context['timed_out'] ?? false)) {
                throw new RonTimeoutException(
                    'Request to RON-CORE gateway timed out.',
                    null,
                    null,
                    null,
                    null,
                    $e
                );
            }

            throw new RonNetworkException(
                'Network error while calling RON-CORE gateway.',
                null,
                null,
                null,
                null,
                $e
            );
        }

        $status = $res->getStatusCode();

        $normalizedHeaders = [];
        foreach ($res->getHeaders() as $name => $values) {
            $normalizedHeaders[$name] = $values;
        }

        return new Response(
            $status,
            $normalizedHeaders,
            (string) $res->getBody()
        );
    }
}
