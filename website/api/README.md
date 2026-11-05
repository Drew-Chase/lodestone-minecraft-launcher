# Lodestone website API

The PHP 8.2 (Slim 4) JSON API served under `/api`. The site's frontend and, later, the launcher depend on the shapes in this file. Change it in the same commit as any route that changes.

## Layout

- `index.php`: entry point. Runs `bootstrap.php`, then flushes queued account mail.
- `bootstrap.php`: loads configuration, builds the Slim app and registers the routes.
- `routes/`: HTTP routes (namespace `routes`).
- `lib/`: application code (namespace `app`, PSR-4 at `api/lib/`).
- `migrations/`: numbered SQL files, applied by `scripts/migrate.php`.
- `config/production.php`: written by CI from GitHub secrets. Never committed.
- `.env`: local development settings, created from `.env.example` by `scripts/dev-env.php`. Never committed.
- `var/`: local runtime files, such as development mail. Never committed.

## Configuration

Each key is read from the first source that defines it. The mode is read once, when configuration loads. `APP_ENV=development` reads `.env` and `config/production.php` first, then environment variables, so variables set on a developer's machine cannot redirect the project. Any other mode reads environment variables first, then `config/production.php`, then `.env`. `environment()` returns the same mode that picked the rule.

| Key | Meaning |
|---|---|
| `APP_ENV` | `development`, `test` or `production`. Defaults to `production`. |
| `APP_URL` | Public origin with no trailing slash. Used in email links and as the JWT issuer. |
| `JWT_SECRET` | HMAC key for access tokens. At least 32 bytes. |
| `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USER`, `DB_PASSWORD` | MySQL 8.0 connection. `DB_PORT` defaults to 3306. |
| `MAIL_TRANSPORT` | `smtp`, `file` (writes each message to `MAIL_DIR`) or `memory` (tests). Defaults to `smtp` in production and `file` elsewhere. |
| `MAIL_FROM` | Sender address. Defaults to `noreply@lodestonelauncher.com`. |
| `MAIL_DIR` | Directory for the `file` transport. Defaults to `api/var/mail`. |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD` | Relay for the `smtp` transport. `SMTP_PORT` defaults to 587. |
| `PASSWORD_BREACH_CHECK` | `off` skips the Pwned Passwords check. The test suite sets it. Leave it unset everywhere else. |
| `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET` | Stripe API key and webhook signing secret. A billing endpoint answers `503 billing_unavailable` while a key it needs is missing or empty. |
| `STRIPE_PRICE_SOLO_MONTHLY`, `STRIPE_PRICE_SOLO_YEARLY`, `STRIPE_PRICE_FRIENDS_MONTHLY`, `STRIPE_PRICE_FRIENDS_YEARLY`, `STRIPE_PRICE_UNLIMITED_MONTHLY`, `STRIPE_PRICE_UNLIMITED_YEARLY` | The Stripe price ids for the six paid plan and interval pairs. Checkout and the webhook need all six. |

## Common rules

- Requests and responses are JSON. Errors are `{"error": "<code>"}` with the status in the table below.
- The client address is `REMOTE_ADDR`. In production, nginx sets it from `CF-Connecting-IP`, and only for connections from the Cloudflare ranges listed in `website/deploy/nginx.conf`. The application never reads forwarded headers itself.
- `/api/auth/refresh` and `/api/auth/logout` take one of two client headers. `X-Lodestone-Client: web` reads the refresh token from the cookie. `X-Lodestone-Client: launcher` reads it from the JSON body `{"refresh_token"}`, and the new token comes back in the body, never in a cookie. Any other value, or none, gives `403 client_header_missing`. Browsers cannot send the `web` header cross-site without a CORS preflight, which is what stops cross-site request forgery there.
- Authenticated endpoints take `Authorization: Bearer <access_token>`.
- Access tokens last 900 seconds. Refresh tokens last 30 days idle and 90 days in total.
- `GET /api/releases/latest` and `GET /api/stats` are public site data. They need no authentication and answer 503 when GitHub cannot be reached.

## Error codes

| Code | Status | Meaning |
|---|---|---|
| `invalid_json` | 400 | The body is not a JSON object. |
| `missing_field` | 400 | A required field is absent or not a string (password, current password, new password). |
| `invalid_email` | 400 | The address is empty, too long, or not an address. |
| `invalid_display_name` | 400 | Longer than 64 characters, or contains control characters. |
| `password_too_short` | 400 | Fewer than 12 characters. |
| `password_too_long` | 400 | More than 256 bytes. |
| `password_breached` | 400 | The password appears in the Pwned Passwords list. |
| `invalid_token` | 400 | The email token is absent, unknown, already used, or for another purpose. |
| `token_expired` | 400 | The email token is past its expiry. |
| `invalid_credentials` | 401 | Wrong email or password, or wrong current password. |
| `unauthorized` | 401 | The access token is missing, malformed, expired or forged. |
| `refresh_invalid` | 401 | No refresh cookie, or the token is unknown or revoked, or it was used within the last 10 seconds. |
| `refresh_reused` | 401 | A refresh token used more than 10 seconds ago came back. Its whole session family is revoked. |
| `refresh_expired` | 401 | The refresh token passed its idle or total lifetime. |
| `cannot_revoke_current` | 409 | The session asked to be revoked is the caller's own. Use logout instead. |
| `client_header_missing` | 403 | `X-Lodestone-Client: web` was not sent. |
| `not_found` | 404 | No such route. |
| `method_not_allowed` | 405 | The route exists with another method. |
| `http_error` | any | Another framework error. The status is the framework's. |
| `rate_limited` | 429 | Too many attempts. The `Retry-After` header gives seconds to wait. |
| `invalid_username` | 400 | The username breaks the rules: 3 to 16 characters from A-Z, a-z, 0-9 and underscore. |
| `username_taken` | 409 | Another account has that username (case-insensitive). |
| `username_required` | 409 | The account has no username yet. Set one first. |
| `email_not_verified` | 403 | The route needs a verified email. |
| `invalid_device` | 400 | The device name, os or app version is empty, too long, or has control characters. |
| `invalid_device_code` | 400 | The device code is unknown. |
| `authorization_pending` | 400 | The code is not approved yet. Poll again. |
| `slow_down` | 400 or 429 | Polled too soon. Device sign-in answers 400 with `Retry-After: 5`. Minecraft answers 429 with the interval as `Retry-After`. |
| `access_denied` | 400 | The sign-in was denied. |
| `expired_token` | 400 | The device code expired while pending, or it was already redeemed. |
| `invalid_code` | 404 | The user code is unknown or malformed. |
| `code_expired` | 410 | The user code has expired. |
| `code_used` | 409 | The user code was already approved or redeemed. |
| `device_limit_reached` | 409 | The plan's device allowance is full. |
| `invalid_friend` | 400 | The target is the caller, or the caller's own invite. |
| `already_friends` | 409 | The two accounts are already friends. |
| `request_pending` | 409 | A request from the caller is already waiting for an answer. |
| `invalid_invite` | 404 | The invite token is unknown. |
| `invite_expired` | 410 | The invite has passed its seven days. |
| `invite_used` | 409 | The invite was already accepted. |
| `invalid_plan` | 400 | The plan or interval is unknown. |
| `billing_unavailable` | 503 | Stripe is not configured for the endpoint. |
| `billing_failed` | 502 | Stripe refused or failed the request. Only the exception class is logged. |
| `already_subscribed` | 409 | The account already has a trialing, active or past_due subscription. Manage it in the portal. |
| `no_billing_account` | 404 | The account has no Stripe customer yet. |
| `invalid_signature` | 400 | A webhook signature is absent, wrong, or older than 300 seconds. |
| `minecraft_unavailable` | 502 | Microsoft, Xbox or Minecraft could not be reached when a link started. Inside a link status it appears as `"status": "failed"` with status 200. |
| `internal_error` | 500 | Unexpected failure. Details go to the server log only. |

## Endpoints

| Method | Path | Auth | Body | Success |
|---|---|---|---|---|
| POST | `/api/auth/register` | none | `{"email", "password", "display_name"?}` | 202 `{"status": "check_email"}` |
| POST | `/api/auth/verify-email` | none | `{"token"}` | 200 `{"status": "verified"}` |
| POST | `/api/auth/resend-verification` | none | `{"email"}` | 202 `{"status": "check_email"}` |
| POST | `/api/auth/login` | none | `{"email", "password"}` | 200 session object, and sets the refresh cookie |
| POST | `/api/auth/refresh` | cookie and client header | none | 200 session object, and rotates the cookie |
| POST | `/api/auth/logout` | cookie and client header | none | 204, and clears the cookie |
| POST | `/api/auth/forgot-password` | none | `{"email"}` | 202 `{"status": "check_email"}` |
| POST | `/api/auth/reset-password` | none | `{"token", "password"}` | 200 `{"status": "reset"}` |
| POST | `/api/auth/change-password` | Bearer | `{"current_password", "new_password"}` | 200 `{"status": "changed"}` |
| GET | `/api/me` | Bearer | none | 200 `{"user": user}` |
| GET | `/api/releases/latest` | none | none | 200 release data (public site data) |
| GET | `/api/stats` | none | none | 200 repository statistics (public site data) |

Session object: `{"access_token": "<jwt>", "token_type": "Bearer", "expires_in": 900, "user": user}`.

User object: `{"id": 1, "email": "a@example.com", "email_verified": true, "display_name": null, "username": null, "presence_hidden": false, "created_at": "2026-11-05T12:00:00+00:00"}`. Times are UTC, in ISO 8601.

Refresh cookie: `__Host-lodestone_refresh=<token>; Max-Age=2592000; Path=/; HttpOnly; Secure; SameSite=Strict`. Clearing it sends the same name with an empty value and `Max-Age=0`. The browser sends it with every request to the site, and only `/api/auth/refresh` and `/api/auth/logout` read it.

Email links, built from `APP_URL`. The token is in the URL fragment, so browsers never send it to the server and web server logs never record it:
- `/verify-email#token=<token>`
- `/reset-password#token=<token>`

Tokens are 64 hex characters.

## Account and session rules

- Sign-in works for unverified accounts. Verification is required before billing, friend requests and sharing (later phases).
- `register` returns the same 202 whether or not the address exists. The address decides which mail is sent:
  - A new address gets an account with the chosen password and a verification link, valid for 24 hours.
  - A verified address gets a notice that someone tried to register it. Nothing else changes.
  - An unverified address keeps its existing password. It gets a "finish registration" link instead: a reset link, valid for 24 hours, that sets a new password and verifies the address. A second registration therefore never changes the password of an account that someone else may have created.
- `resend-verification` sends the same "finish registration" link to an unverified address. For any other address it sends nothing. It returns the same 202 either way.
- `forgot-password` sends a reset link only for an existing, undeleted account. It returns the same 202 either way.
- Verification lasts 24 hours and a reset link lasts 1 hour. Each is single-use. Requesting a new one invalidates the earlier unused ones.
- A verification revokes every session of the account.
- A password reset (including a finish-registration link) sets the password, verifies the address, and revokes every session.
- A password change revokes every other session and keeps the current one.
- Refresh tokens rotate on every use. A token used within the last 10 seconds returns `refresh_invalid` and revokes nothing, so two tabs that refresh at once do not sign each other out. A token used earlier than that revokes its whole session family.
- Access tokens are stateless. A revoked session's access token stays valid until it expires, at most 15 minutes.
- Account mail is sent after the response has been returned. Database work still differs by a few milliseconds between an address with an account and one without, so response times are not perfectly uniform. The response body and status never differ.
- Accepted risk, decided 2026-11-05: a stranger who registers an address that belongs to someone else keeps a working password after the owner verifies that address, because verification does not change the password. The owner can end that access at any time by requesting a password reset and setting a new password, which revokes every session.

## Rate limits

Counts are per fixed window. Bucket keys are stored hashed.

| Action | Limit | Window |
|---|---|---|
| `register` per client address | 10 attempts | 1 hour |
| `login` failures per address | 5 failures | 15 minutes |
| `login` failures per client address | 5 failures | 15 minutes |
| `resend-verification` per address | 3 attempts | 1 hour |
| `resend-verification` per client address | 10 attempts | 1 hour |
| `forgot-password` per address | 3 attempts | 1 hour |
| `forgot-password` per client address | 10 attempts | 1 hour |
| `verify-email` per client address | 30 attempts | 15 minutes |
| `reset-password` per client address | 30 attempts | 15 minutes |
| `change-password` failures per user | 5 failures | 15 minutes |
| SSO `start` per client address | 30 attempts | 15 minutes |
| Device sign-in `start` per client address | 10 attempts | 15 minutes |
| Device approval per user | 30 attempts, every attempt counted | 15 minutes |
| Device lookup per user | 30 lookups | 15 minutes |
| Friend search per user | 30 searches | 15 minutes |
| Friend requests per user | 30 requests | 15 minutes |
| Friend invites per user | 30 invites | 15 minutes |
| Checkout starts per user | 10 attempts | 1 hour |
| Minecraft link starts per user | 10 attempts | 1 hour |

A successful login clears the address's login failure count.

## Passwords

- At least 12 characters, counted as characters. At most 256 bytes. No composition rules.
- Rejected when the password appears in the Pwned Passwords list. The check uses the k-anonymity range API, so only the first five characters of the SHA-1 hash leave the server. If the service cannot be reached, the password is accepted and the failure is logged.
- Stored with argon2id. When PHP has no argon2, bcrypt is used over the base64 SHA-256 digest of the password, so the whole 256 bytes count rather than the first 72.

## Sessions

A session is one refresh-token family: one sign-in and every token rotated from it.

- `GET /api/account/sessions` (Bearer) returns `200 {"sessions": [...]}`. Each item is `{"id": "<family id>", "client": "web", "created_at": "<ISO>", "last_used_at": "<ISO>", "current": true|false}`. It lists the account's active sessions: families with at least one unrevoked, unexpired token. `current` is true for the session of the access token that made the request.
- `DELETE /api/account/sessions/{id}` (Bearer) returns `204` and revokes that session. It returns `404 not_found` when the id is not one of the account's active sessions. It returns `409 cannot_revoke_current` when the id is the caller's own session: use `POST /api/auth/logout` for that.
- `POST /api/account/sessions/revoke-others` (Bearer) returns `200 {"status": "revoked", "count": <int>}` and revokes every session except the caller's.

## Single sign-on

Three providers: Google (OpenID Connect), Microsoft personal accounts (OpenID Connect on the `consumers` endpoint) and GitHub (OAuth). A provider is enabled only when both its client ID and its client secret are configured:

| Key | Meaning |
|---|---|
| `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` | Google OAuth client (Web application). |
| `MICROSOFT_CLIENT_ID`, `MICROSOFT_CLIENT_SECRET` | Microsoft Entra app for personal accounts. The secret is its value. |
| `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET` | GitHub OAuth App for production. Each GitHub OAuth App takes one callback URL. |

The callback URL registered with each provider is `{APP_URL}/api/auth/{provider}/callback`, where `{provider}` is `google`, `microsoft` or `github`.

Endpoints:
- `GET /api/auth/providers` returns `200 {"providers": ["google", ...]}`, listing only the enabled providers. It never returns secrets, and it needs no authentication.
- `GET /api/auth/{provider}/start?next=/path` returns `302` to the provider's sign-in page and sets the `__Host-lodestone_oauth_state` cookie (see the rules below). `next` must be a path that begins with a single `/`, with no backslash and no control characters, at most 255 bytes and valid UTF-8. Anything else becomes `/account`. An unknown or disabled provider returns `404 not_found`. A client address may start 30 sign-ins in 15 minutes. More return `429 rate_limited`.
- `GET /api/auth/{provider}/callback?code=…&state=…` returns `302`. On success it sets the refresh cookie and redirects to `{APP_URL}{next}`. The account page exchanges that cookie for an access token through `/api/auth/refresh` when it loads. Other pages do not refresh on load. On failure it redirects to `{APP_URL}/login?error=<code>`, using the codes below. An unknown or disabled provider returns `404 not_found`, as at start. A callback whose state does not match the cookie is refused and leaves the cookie alone. Every other redirecting callback clears the cookie, whatever the outcome.

The callback always redirects, apart from its `404 not_found` and the usual `500 internal_error` when the server cannot run (for example, when `APP_URL` is unset). Start returns JSON for `404 not_found` and `429 rate_limited`, and for the usual `500 internal_error` if something fails unexpectedly.

Rules:
- Each start creates a single-use state that expires after 10 minutes, with a PKCE verifier and an OpenID nonce. Only the SHA-256 of the state is stored. About one start in fifty also deletes states that expired more than a day ago. The callback accepts only an unused, unexpired state.
- The state is bound to the browser that started the sign-in. Start sets `__Host-lodestone_oauth_state=<state>; Max-Age=600; Path=/; HttpOnly; Secure; SameSite=Lax`. The `__Host-` prefix makes the browser refuse the cookie unless it is Secure, has `Path=/` and has no `Domain`, so a sibling subdomain cannot plant one. Otherwise the callback is refused with `oauth_state` unless that cookie is present and equals the `state` query value, compared in constant time. That check runs before the provider's `error` parameter is read, so a link without the bound state cannot cancel a sign-in in progress. The cookie is `Lax` rather than `Strict` because the provider returns the browser with a cross-site top-level `GET`, and `Strict` cookies are not sent on that request. `Lax` still withholds the cookie from cross-site `POST` requests.
- A second sign-in started in the same browser replaces the cookie, so the first callback fails with `oauth_state`. A sign-in started in one browser and completed in another fails the same way.
- Google and Microsoft ID tokens are checked for signature (against the provider's published keys), issuer, audience (the client ID), expiry and nonce.
- Sign-in requires a verified email from the provider. For Google that is `email_verified`. For GitHub it is a primary, verified address from `/user/emails`. For Microsoft personal accounts the `email` claim is accepted, because Microsoft verifies those addresses.
- An existing identity (provider and provider subject) signs in directly.
- Otherwise, if a local account has the same email:
  - it is linked when that account is verified;
  - it is not linked when that account is unverified (`oauth_account_unverified`). An unverified account may hold a password chosen by someone else, so it must be verified through the finish-registration or reset flow first.
- Otherwise a new verified account is created with no password. It is linked to the provider identity.
- Every SSO sign-in starts a `web` session, like a password sign-in, and is audited as `login_sso`.
- The Microsoft issuer and its `email` claim are not yet checked against a live personal Microsoft sign-in. Check both before go-live.
- The refresh cookie is `SameSite=Strict`, and the callback sets it on a response to a cross-site navigation from the provider. Whether every browser keeps it there has not yet been checked against a live provider. Check it before go-live.

Callback error codes, sent as `?error=` on the login page:

| Code | Meaning |
|---|---|
| `oauth_denied` | The person refused, or the provider reported an error. |
| `oauth_state` | The state is missing, already used, expired, or not bound to this browser. |
| `oauth_token` | The code is missing or could not be exchanged for tokens, a provider endpoint failed or answered unusably, or the server failed unexpectedly. The server log records only the exception class. |
| `oauth_identity` | The provider gave no usable, verified identity, or the linked account is deleted. |
| `oauth_account_unverified` | The matching local account is not verified, so it was not linked. |

## Usernames

- 3 to 16 characters, from A-Z, a-z, 0-9 and underscore. Unique and case-insensitive. Chosen after sign-up, not required to sign in, and changeable.
- `POST /api/account/username` (Bearer) with `{"username"}` returns `200 {"username"}`. Errors: `400 missing_field`, `400 invalid_username`, `409 username_taken`.
- Friend features need a username. See Friends below.

## Devices and launcher sign-in

A device is one launcher installation. The launcher starts a sign-in, the person approves it on the website while signed in, and the approval creates the device and its refresh family.

- `POST /api/device/start` (no auth) with `{"name", "os", "app_version"}`: each field is 1 to 64, 1 to 32 and 1 to 32 characters after trimming, with no control characters. Returns `200` with `device_code`, `user_code`, `verification_uri` (`{APP_URL}/link`), `verification_uri_complete` (the same with `?code=`), `expires_in: 600` and `interval: 5`.
  - `device_code` is 32 random bytes in base64url. It is returned once, and only its SHA-256 is stored.
  - `user_code` is 8 characters from `ABCDEFGHJKMNPQRSTVWXYZ23456789`, shown as `XXXX-XXXX`.
- `POST /api/device/approve` (Bearer; no verified email needed, since sign-in works for unverified accounts) with `{"user_code"}`. The hyphen is optional and the case does not matter. Returns `200 {"status": "approved"}`. Errors: `404 invalid_code`, `410 code_expired`, `409 code_used`, `409 device_limit_reached`. Every attempt counts toward the limit, including failed ones.
- `POST /api/device/lookup` (Bearer) with `{"user_code"}` returns the device a pending code belongs to, before it is approved: `200 {"name", "os", "app_version", "expires_at"}`. It never returns the device code or its hash. Errors: `404 invalid_code`, `410 code_expired`, `409 code_used`. Rate limit: 30 lookups per user per 15 minutes. Any signed-in account that holds the code can see these details, which is the same exposure approve has.
- `POST /api/device/token` (no auth) with `{"device_code"}`. The launcher polls no faster than every 5 seconds.
  - Pending returns `400 authorization_pending`.
  - A poll less than 5 seconds after the previous one returns `400 slow_down` with `Retry-After: 5`.
  - A denied sign-in returns `400 access_denied`.
  - A code that expired while pending, or was already redeemed, returns `400 expired_token`.
  - An unknown code returns `400 invalid_device_code`.
  - An approved code returns `200 {"access_token", "token_type": "Bearer", "expires_in": 900, "refresh_token", "user", "device": {"id", "name"}}`, once. The refresh token is in the body. The launcher keeps it in the OS keyring.
  - Redeeming checks the device limit again. `409 device_limit_reached` leaves the approval in place.
- `GET /api/devices` (Bearer) returns the active devices, newest first: `{"devices": [{"id", "name", "os", "app_version", "created_at", "last_seen_at"}], "limit"}`. `limit` is the plan's device allowance, and `null` means unlimited.
- `DELETE /api/devices/{id}` (Bearer) revokes the device and every refresh family issued to it, and returns `204`. Access tokens already issued stay valid until they expire, at most 15 minutes. Errors: `404 not_found` for another account's device, a revoked device, or a malformed id.
- Launcher refresh: `POST /api/auth/refresh` with `X-Lodestone-Client: launcher` and the body `{"refresh_token"}`. Rotation follows the web rules, including the 10-second grace window and reuse detection. Returns the session object with the new `refresh_token` in the body, and updates the device's `last_seen_at`.
- Launcher logout: `POST /api/auth/logout` with `X-Lodestone-Client: launcher` and the same body. Revokes the family and returns `204`.
- A token issued to another client is refused on both launcher paths with `401 refresh_invalid`, and nothing is rotated or revoked.

## Friends

Friend features need a verified email (`403 email_not_verified`). Sending requests by username or code, searching, creating invites, accepting invites and accepting requests also need a username (`409 username_required`). Friend codes, code reset, the list and removal need neither.

- `GET /api/friends/code` (Bearer) returns `200 {"code"}`, an 8-character code created on first use. It is shown as `XXXX-XXXX`, and it can be entered with or without the hyphen. `POST /api/friends/code/reset` returns a new code and retires the old one.
- `GET /api/friends/search?username=` (Bearer) is an exact, case-insensitive match against another account with a username. Returns `200 {"user": {"username"}}`, or `404 not_found` (also when the match is the caller).
- `POST /api/friends/requests` (Bearer) with exactly one of `{"username"}` or `{"code"}`, otherwise `400 missing_field`. Returns `201 {"request": {"id", "status": "pending", "username"}}`. If the other account had already sent a request, the two become friends at once and the response is `200 {"status": "accepted", "friend": {"username"}}`. Errors: `404 not_found`, `400 invalid_friend`, `409 already_friends`, `409 request_pending`.
- `GET /api/friends` (Bearer) returns `{"friends": [...], "incoming": [...], "outgoing": [...]}`. Each item is `{"id", "username"}`, where `id` is the friendship id. Only usernames are shown, never emails. `username` is `null` for a friend who has no username.
- `POST /api/friends/requests/{id}/accept` (Bearer) is open only to the addressee of a pending request. Returns `200 {"status": "accepted"}`, or `404 not_found`.
- `DELETE /api/friends/{id}` (Bearer) removes a friendship, cancels a request the caller sent, or declines one the caller received. Returns `204`, or `404 not_found`.
- `POST /api/friends/invites` (Bearer) returns `201 {"url", "expires_at"}`. The url is `{APP_URL}/friends/invite#token=<token>`. The token sits in the fragment, so it never reaches a query string. An invite is single-use and lasts seven days.
- `POST /api/friends/invites/accept` (Bearer) with `{"token"}` returns `200 {"status": "accepted", "friend": {"username"}}`. Errors: `404 invalid_invite`, `410 invite_expired`, `409 invite_used`, `400 invalid_friend` (the caller's own invite), `409 already_friends`.

Deleted accounts are excluded from search, request targets and friend lists.

## Billing

Stripe Checkout starts a subscription with a seven-day trial, and a card is always required. The customer portal manages it afterwards. Free is the default and needs no subscription row. Storage is in bytes, with decimal gigabytes (1 GB is 1,000,000,000 bytes).

| Plan | Storage | Devices | Shares | Intervals |
|---|---|---|---|---|
| free | 0 | 1 | 0 | none |
| solo | 100,000,000,000 | 5 | 0 | monthly, yearly |
| friends | 250,000,000,000 | 5 | 5 | monthly, yearly |
| unlimited | 500,000,000,000 | unlimited | unlimited | monthly, yearly |

| Method | Path | Auth | Body | Success |
|---|---|---|---|---|
| GET | `/api/billing/plans` | none | none | 200 `{"plans": [...]}`, paid plans only |
| POST | `/api/billing/checkout` | Bearer | `{"plan", "interval"}` | 200 `{"url"}`, the Checkout session URL |
| POST | `/api/billing/portal` | Bearer | none | 200 `{"url"}`, the portal session URL |
| GET | `/api/billing/subscription` | Bearer | none | 200 `{"subscription": {...}}` |
| POST | `/api/billing/webhook` | `Stripe-Signature` header | the raw Stripe event | 200 `{"status": "processed" \| "ignored" \| "duplicate"}` |

The subscription object holds the plan's allowances and Stripe's state: `plan`, `name`, `status`, `interval`, `storage_bytes`, `devices`, `shares`, `trial_ends_at`, `current_period_end` and `cancel_at_period_end`. A plan is entitled while its status is `trialing`, `active` or `past_due`. `past_due` keeps the allowances while Stripe retries the payment. The other Stripe statuses (`incomplete`, `incomplete_expired`, `unpaid`, `paused` and `canceled`) give the free plan.

Checkout checks, in this order: the Stripe key and price configuration (503 `billing_unavailable`), the body (400 `missing_field`, or 400 `invalid_plan` for an unknown plan or interval), a verified email (403 `email_not_verified`), and no trialing, active, past_due or incomplete subscription (409 `already_subscribed`). Only starts that pass these checks count toward the limit of 10 per user per hour.

The portal checks the configuration (503) before the customer (404 `no_billing_account`). Checkout creates the Stripe customer once and reuses it. The session has a seven-day trial, a card always required, the user id as `client_reference_id`, and success and cancel URLs under `{APP_URL}` (`/dashboard/billing?checkout=success` and `/subscribe?checkout=cancelled`). The portal returns to `/dashboard/billing`.

The webhook:
- The signature must verify within 300 seconds, or the answer is 400 `invalid_signature`. The Stripe keys must be configured, or the answer is 503.
- Each event is recorded in `stripe_events` in the same transaction that applies it. A repeated event id answers `duplicate` and changes nothing. A failure rolls the transaction back, so Stripe's retry applies the event again.
- A subscription's state always comes from a fresh Stripe retrieve, not from the event payload, because Stripe does not guarantee event order.
- A customer has at most one live subscription. The user's row is locked while an event is applied. If the row holds a live subscription and another live one arrives, the new one is cancelled. If the held subscription is not live, the row switches to the new one.
- `checkout.session.completed` links the customer and the subscription. When the account no longer exists, or the session's customer does not match the row, a live subscription is cancelled and nothing is written.
- `customer.subscription.created`, `customer.subscription.updated` and `customer.subscription.deleted` update the row found by customer. `invoice.paid` and `invoice.payment_failed` find the row by the invoice's customer and apply the retrieved state of its subscription. Other events are recorded and answer `ignored`.
- Stripe requests use a 10-second timeout and a 5-second connect timeout, because the user's row stays locked while they run.

Required keys: checkout needs `STRIPE_SECRET_KEY` and the six price keys; the portal needs `STRIPE_SECRET_KEY`; the webhook needs `STRIPE_WEBHOOK_SECRET` and the six price keys. The plans and subscription reads need none.

Errors: `invalid_plan` 400, `missing_field` 400, `invalid_signature` 400, `email_not_verified` 403, `no_billing_account` 404, `already_subscribed` 409, `billing_unavailable` 503, `billing_failed` 502 (only the exception class is logged), and `rate_limited` 429.

## Minecraft accounts

A user links Minecraft Java accounts through Microsoft's device-code sign-in, using the launcher's Microsoft app. Every route needs a Bearer token for a live account. Email verification is not required.

| Method | Path | Success |
|---|---|---|
| POST | `/api/minecraft/link` | 201 `{"id", "user_code", "verification_uri", "expires_at", "interval"}` |
| GET | `/api/minecraft/link/{id}` | 200 status object (below) |
| GET | `/api/minecraft/accounts` | 200 `{"accounts": [{"uuid", "name", "linked_at"}]}` |
| DELETE | `/api/minecraft/accounts/{uuid}` | 204 |

- Starting a link is limited to 10 per user per hour. It deletes the user's earlier pending sessions.
- The person enters `user_code` at `verification_uri`. The client polls the status route no faster than `interval` seconds. A faster poll answers 429 `slow_down` with `Retry-After`.
- A status object is one of:
  - `{"status": "pending", "interval": n}`: keep polling;
  - `{"status": "expired"}`;
  - `{"status": "denied"}`;
  - `{"status": "linked", "account": {"uuid", "name", "linked_at"}}`;
  - `{"status": "failed", "error": "minecraft_not_owned" | "minecraft_restricted" | "minecraft_linked_elsewhere" | "minecraft_unavailable"}`.
- A session that ends with any status is deleted, so a later poll answers `404 not_found`.
- `minecraft_not_owned`: the account has no Xbox Live profile (XErr 2148916233), or no Minecraft profile.
- `minecraft_restricted`: Xbox Live restricts the account in its country or region (XErr 2148916234 and 2148916235), needs adult verification (2148916236 and 2148916237), or is a child account that is not yet in a family (2148916238).
- `minecraft_linked_elsewhere`: another Lodestone account already holds that Minecraft account.
- `minecraft_unavailable`: a service failed, or Microsoft reported that the device code is invalid.
- Microsoft's poll answers: `authorization_pending` keeps the session pending; `slow_down` adds 5 seconds to the interval; `authorization_declined` or `access_denied` gives denied; `expired_token` gives expired. `bad_verification_code` and `invalid_grant` end the session as `minecraft_unavailable`. Any other unknown client error, a network failure and a 5xx keep the session for the next poll, until it expires.
- Microsoft's device code is stored in `minecraft_link_sessions.device_code`, because the server must poll with it. It lasts at most 15 minutes and is deleted when the session ends. Expired sessions are swept at about one start in twenty. Microsoft, Xbox and Minecraft tokens are never stored. They are used for the chain and then dropped.
- Profile names must match `^[A-Za-z0-9_]{1,16}$`. A uuid is stored in its dashed form.

Errors on the routes themselves: `not_found` 404, `slow_down` 429, `rate_limited` 429 on start, and `minecraft_unavailable` 502 on start when Microsoft cannot be reached.

## Running the tests

From `website/`, run `php vendor/bin/phpunit`. The tests use the database named in `DB_NAME`, which must end in `_test`, and they refuse a remote host. `just db-up` starts the Docker MySQL that provides it. On Windows, enable `extension=pdo_mysql` in `php.ini` first.
