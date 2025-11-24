<?php

declare(strict_types=1);

// Composer autoloader (PSR-4 for the SDK namespace).
require __DIR__ . '/../vendor/autoload.php';

// Ensure pagination helpers are loaded (Page + Paginator live in Pagination.php).
// This sidesteps any PSR-4 filename assumptions for multi-class files.
require __DIR__ . '/../src/Pagination.php';
