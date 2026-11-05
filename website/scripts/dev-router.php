<?php

declare(strict_types=1);

// Development router for the PHP server (see the dev:api script in package.json).
// `php -S` has no rewrite rules, so this does what .htaccess and nginx do in production:
// real files are served as they are, and everything else goes to the Slim app.

$path = parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH) ?: '/';

// Refuse dotfiles and the API's private folders before anything is served. The path is
// decoded and normalised first, so an encoded or doubled-slash form cannot slip past.
$normalised = strtolower(preg_replace('#/+#', '/', rawurldecode($path)));
if (
    preg_match('#(^|/)\.#', $normalised) === 1
    || str_starts_with($normalised, '/api/var/')
    || str_starts_with($normalised, '/api/config/')
) {
    http_response_code(404);
    header('Content-Type: application/json');
    echo '{"error":"not_found"}';
    return true;
}

if ($path !== '/' && !str_ends_with($path, '.php') && is_file($_SERVER['DOCUMENT_ROOT'] . $path)) {
    return false;
}

require $_SERVER['DOCUMENT_ROOT'] . '/index.php';
