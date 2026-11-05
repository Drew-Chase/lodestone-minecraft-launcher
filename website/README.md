# Lodestone website

The site at lodestonelauncher.com. It is a React 19 + TypeScript frontend (Vite) with a PHP 8.2 API (Slim 4), built from the TAVERN PHP template.

- `src/`: the one-page site. Sections are in `src/components`, and the release lookup is `src/hooks/useRelease.ts`.
- `api/index.php`: the Slim app. `api/routes/ReleaseRoute.php` serves `GET /api/releases/latest` and `GET /api/stats`. `api/lib/GitHubReleases.php` reads the GitHub API and caches the result for 30 minutes in the system temp folder.
- `public/`: files served as they are (favicons, OG image, `robots.txt`, `sitemap.xml`).
- `/install` serves `res/installer.sh` (the repo's Linux installer, shipped with each release). `vite.config.ts` serves it in dev and copies it to `dist/install` at build, so `curl -fsSL https://lodestonelauncher.com/install | sh` runs the same file as the release.
- `deploy/nginx.conf`: the server block for the VPS, installed once (see Deploy).

## Develop

You need Node 22+, pnpm 10, Composer, and PHP 8.2+ with the `curl` extension.

```bash
just install   # or: composer install && pnpm install
just dev       # Vite on http://localhost:3000, PHP API on http://localhost:8000
```

Vite proxies `/api` to the PHP server. `scripts/dev-router.php` gives `php -S` the same routing that nginx gives production.

## Build

```bash
just build     # writes dist/: the site, plus api/ and vendor/
```

## Deploy

`.github/workflows/website.yml` builds the site for every change under `website/` and to `res/installer.sh`. When the change lands on `master`, the workflow uploads `dist/` to `lodestonelauncher.com` over FTP. It reads two repository secrets: `WEBSITE_FTP_USERNAME` and `WEBSITE_FTP_PASSWORD`. The FTP login's starting folder is the web root, so the upload goes to `./`.

The server runs nginx, not Apache, so `.htaccess` is not used. Install `deploy/nginx.conf` once:

1. Set `root` to the FTP login's folder, and point the SSL lines at the certificate the server already uses.
2. Merge the server blocks into the existing nginx configuration.
3. Run `sudo nginx -t && sudo systemctl reload nginx`.

## Notes

- The API returns 503 when GitHub is unreachable and no cached copy exists. The site then shows "Coming Soon", as it did before.
- Release files are matched by file name in `platform()` in `api/lib/GitHubReleases.php`. Update those rules when the release asset names change.
- The workflow pins the FTP action to a commit, because the action receives the FTP password.
