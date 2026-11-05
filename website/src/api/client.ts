import {ApiError} from "./errors.ts";
import {getAccessToken, setSignedIn, setSignedOut} from "./session.ts";
import type {Session} from "./types.ts";

const REFRESH_PATH = "/api/auth/refresh";
const LOGOUT_PATH = "/api/auth/logout";
// Cookie-authenticated endpoints must carry this header. A cross-site page cannot send it without a CORS preflight.
const CLIENT_HEADER_PATHS = new Set([REFRESH_PATH, LOGOUT_PATH]);

export interface ApiOptions {
    method?: "GET" | "POST" | "DELETE";
    body?: unknown;
    /** True for calls that need the access token. A 401 on one of these refreshes the session and retries once. */
    auth?: boolean;
}

function parseRetryAfter(value: string | null): number | undefined {
    if (!value) return undefined;
    const seconds = Number(value);
    if (Number.isFinite(seconds) && seconds >= 0) return Math.ceil(seconds);
    const date = Date.parse(value);
    if (Number.isNaN(date)) return undefined;
    return Math.max(0, Math.ceil((date - Date.now()) / 1000));
}

function errorCodeOf(data: unknown): string | null {
    if (typeof data === "object" && data !== null && "error" in data && typeof data.error === "string") {
        return data.error;
    }
    return null;
}

async function send<T>(path: string, options: ApiOptions): Promise<T> {
    const headers: Record<string, string> = {Accept: "application/json"};
    if (options.body !== undefined) headers["Content-Type"] = "application/json";
    if (CLIENT_HEADER_PATHS.has(path)) headers["X-Lodestone-Client"] = "web";
    const token = getAccessToken();
    if (token) headers.Authorization = `Bearer ${token}`;

    let response: Response;
    let text: string;
    try {
        response = await fetch(path, {
            method: options.method ?? "GET",
            headers,
            body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
            credentials: "include",
        });
        text = await response.text();
    } catch {
        throw new ApiError("network_error", 0);
    }

    const retryAfter = parseRetryAfter(response.headers.get("Retry-After"));
    let data: unknown = undefined;
    if (text.length > 0) {
        try {
            data = JSON.parse(text);
        } catch {
            throw new ApiError("internal_error", response.status, retryAfter);
        }
    }

    if (!response.ok) {
        throw new ApiError(errorCodeOf(data) ?? "internal_error", response.status, retryAfter);
    }
    return data as T;
}

// One refresh at a time. Concurrent callers share the same promise, because a refresh token is spent once.
let refreshing: Promise<Session> | null = null;

// Codes that mean this browser no longer holds a usable session. Any other failure, such as a network error, leaves the
// signed-in state as it was.
const SESSION_ENDED_CODES = new Set(["refresh_invalid", "refresh_reused", "refresh_expired", "unauthorized"]);

// A refresh_invalid can come from a second tab that rotated the token a moment earlier, so it gets one more attempt
// after a short wait before the session is treated as ended.
const REFRESH_RETRY_DELAY_MS = 600;

export function isSessionEnded(error: unknown): boolean {
    return error instanceof ApiError && SESSION_ENDED_CODES.has(error.code);
}

function delay(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

async function requestRefresh(): Promise<Session> {
    try {
        return await send<Session>(REFRESH_PATH, {method: "POST"});
    } catch (error) {
        if (!(error instanceof ApiError && error.code === "refresh_invalid")) throw error;
    }
    await delay(REFRESH_RETRY_DELAY_MS);
    return send<Session>(REFRESH_PATH, {method: "POST"});
}

/**
 * Trades the refresh cookie for a new session. The browser is marked signed out only when the session has ended
 * (see `isSessionEnded`). A network failure keeps the current state and is thrown to the caller.
 */
export function refreshSession(): Promise<Session> {
    if (!refreshing) {
        refreshing = requestRefresh()
            .then((session) => {
                setSignedIn(session);
                return session;
            }, (error: unknown) => {
                if (isSessionEnded(error)) setSignedOut();
                throw error;
            })
            .finally(() => {
                refreshing = null;
            });
    }
    return refreshing;
}

/** Calls the API. Failures are thrown as `ApiError`. */
export async function apiRequest<T>(path: string, options: ApiOptions = {}): Promise<T> {
    try {
        return await send<T>(path, options);
    } catch (error) {
        // Only a rejected access token is retried. A 401 such as `invalid_credentials` is an answer from the endpoint.
        const retryable = options.auth === true && error instanceof ApiError && error.code === "unauthorized";
        if (!retryable) throw error;
        await refreshSession();
        return send<T>(path, options);
    }
}

/** Signs out on the server. The cookie is cleared even when the call fails, so callers may ignore the error. */
export function logout(): Promise<void> {
    return apiRequest<void>(LOGOUT_PATH, {method: "POST"});
}
