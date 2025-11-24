<?php

declare(strict_types=1);

namespace Ron\AppSdkPhp\Tests\Unit;

use PHPUnit\Framework\TestCase;
use Ron\AppSdkPhp\Page;
use Ron\AppSdkPhp\Paginator;

final class PaginationTest extends TestCase
{
    public function testPaginatorIteratesAcrossPages(): void
    {
        $pages = [
            new Page([1, 2], 'next-1'),
            new Page([3], null),
        ];

        $calls = 0;

        $iter = Paginator::iterate(function (?string $token) use (&$calls, $pages): Page {
            $page = $pages[$calls] ?? new Page([], null);
            $calls++;

            return $page;
        });

        $items = iterator_to_array($iter, false);

        $this->assertSame([1, 2, 3], $items);
        $this->assertSame(2, $calls);
    }
}
