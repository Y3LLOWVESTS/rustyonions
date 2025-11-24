<?php

declare(strict_types=1);

use PhpCsFixer\Config;
use PhpCsFixer\Finder;

/*
 * RO:WHAT  — php-cs-fixer config for ron-app-sdk-php.
 * RO:WHY   — Keep style consistent across src/tests/examples.
 * RO:INV   — PSR-12 base, no risky rules by default.
 */

$finder = (new Finder())
    ->in(__DIR__ . '/src')
    ->in(__DIR__ . '/tests')
    ->in(__DIR__ . '/examples')
;

return (new Config())
    ->setRiskyAllowed(false)
    ->setUnsupportedPhpVersionAllowed(true)
    ->setRules([
        '@PSR12' => true,
        'array_syntax' => ['syntax' => 'short'],
        'ordered_imports' => true,
        'no_unused_imports' => true,
        'no_trailing_whitespace' => true,
        'single_quote' => true,
        'no_extra_blank_lines' => true,
    ])
    ->setFinder($finder);
