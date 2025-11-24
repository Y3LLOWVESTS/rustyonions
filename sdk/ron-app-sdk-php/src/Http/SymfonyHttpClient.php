<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Http;

use Ron\AppSdkPhp\Exception\RonNetworkException;
use Ron\AppSdkPhp\Exception\RonTimeoutException;
use Ron\AppSdkPhp\Response;
use Symfony\Component\HttpClient\HttpClient;
use Symfony\Contracts\HttpClient\Exception\TimeoutExceptionInterface;
use Symfony\Contracts\HttpClient\Exception\TransportExceptionInterface;
use Symfony\Contracts\HttpClient\HttpClientInterface as SymfonyHttpClientInterface;

/**
 * RO:WHAT — HttpClientInterface implementation backed by symfony/http-client.
 * RO:WHY  — Gives SDK users an alternative HTTP backend (besides Guzzle)
 *           while keeping RonClient dependent only on HttpClientInterface.
 * RO:INTERACTS —
 *   * RonClient (primary consumer)
 *   * ClientConfig (may decide which implementation to use)
 * RO:INVARIANTS —
 *   * Never throws Symfony HTTP exceptions directly; always maps to Ron* exceptions.
 *   * Never logs or exposes raw bodies (that’s the caller’s responsibility).
 *   * Uses the timeout passed in (overall) for both request + connect timeouts.
 */
final class SymfonyHttpClient implements HttpClientInterface
{
    private SymfonyHttpClientInterface $client;

    public function __construct(?SymfonyHttpClientInterface $client = null)
    {
        // max_redirects = 0 → no redirects; caller/gateway should control routing.
        $this->client = $client ?? HttpClient::create([
            'max_redirects' => 0,
        ]);
    }

    /**
     * @param array<string,string> $headers
     *
     * @throws RonNetworkException
     * @throws RonTimeoutException
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
            'headers' => $headers,
            'timeout' => $timeoutSeconds,
            // max_duration is a hard cap on the full request lifecycle.
            'max_duration' => $timeoutSeconds,
        ];

        if ($body !== null) {
            $options['body'] = $body;
        }

        try {
            $symfonyResponse = $this->client->request($method, $url, $options);

            // getContent(false) => do NOT throw on 4xx/5xx; we always want the body.
            $statusCode = $symfonyResponse->getStatusCode();
            /** @var array<string,string[]> $responseHeaders */
            $responseHeaders = $symfonyResponse->getHeaders(false);
            $rawBody = $symfonyResponse->getContent(false);
        } catch (TimeoutExceptionInterface $e) {
            // Explicit timeout classification.
            throw new RonTimeoutException(
                'Request to RON-CORE gateway timed out.',
                null,
                null,
                null,
                null,
                $e,
            );
        } catch (TransportExceptionInterface $e) {
            // Any other transport-level failure is a generic network error.
            throw new RonNetworkException(
                'Network error while calling RON-CORE gateway.',
                null,
                null,
                null,
                null,
                $e,
            );
        }

        return new Response($statusCode, $responseHeaders, $rawBody);
    }
}
