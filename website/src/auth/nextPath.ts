/**
 * The `next` query value, if it is a path inside this site. Only a single leading slash is accepted, so `//host` and
 * `/\host` (which browsers read as another host) fall back to the account page.
 */
export function safeNextPath(next: string | null, fallback = "/account"): string {
    if (!next || !next.startsWith("/") || next.startsWith("//") || next.startsWith("/\\")) {
        return fallback;
    }
    return next;
}
