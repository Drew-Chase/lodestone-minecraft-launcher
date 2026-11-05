// Email links carry their token in the URL fragment (`/verify-email#token=<token>`). Browsers never send a fragment to
// the server, so the token stays out of server and proxy logs.
export function tokenFromHash(hash: string): string {
    return new URLSearchParams(hash.replace(/^#/, "")).get("token") ?? "";
}
