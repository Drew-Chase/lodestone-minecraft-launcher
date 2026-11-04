<?php

// Development router for the PHP server (see the dev:api script in package.json).
// `php -S` has no rewrite rules, so this does what .htaccess and nginx do in production:
// real files are served as they are, and everything else goes to the Slim app.

$path = parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH) ?: '/';

if ($path !== '/' && !str_ends_with($path, '.php') && is_file($_SERVER['DOCUMENT_ROOT'] . $path)) {
    return false;
}

require $_SERVER['DOCUMENT_ROOT'] . '/index.php';
