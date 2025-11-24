<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\ClientConfig;
use Ron\AppSdkPhp\Exception\RonConfigException;

final class ClientConfigTest extends TestCase
{
    public function testFromArrayRequiresBaseUrl(): void
    {
        $this->expectException(RonConfigException::class);
        ClientConfig::fromArray([]);
    }

    public function testFromArrayAcceptsMinimalConfig(): void
    {
        $config = ClientConfig::fromArray([
            'baseUrl' => 'https://example.test',
        ]);

        $this->assertSame('https://example.test', $config->getBaseUrl());
        $this->assertNull($config->getToken());
        $this->assertFalse($config->isInsecureHttpAllowed());
    }
}
