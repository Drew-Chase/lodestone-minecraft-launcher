import type {Session, User} from "./types.ts";

// The signed-in state lives here, outside React, so the API client can update it when a refresh succeeds or fails.
// The access token is held in memory only. It is never written to storage.

export type AuthStatus = "loading" | "signed_out" | "signed_in";

export interface SessionState {
    status: AuthStatus;
    user: User | null;
    token: string | null;
}

let state: SessionState = {status: "loading", user: null, token: null};
const listeners = new Set<() => void>();

export function getSessionState(): SessionState {
    return state;
}

export function subscribeSession(listener: () => void): () => void {
    listeners.add(listener);
    return () => {
        listeners.delete(listener);
    };
}

function publish(next: SessionState): void {
    state = next;
    listeners.forEach((listener) => listener());
}

export function getAccessToken(): string | null {
    return state.token;
}

export function setSignedIn(session: Session): void {
    publish({status: "signed_in", user: session.user, token: session.access_token});
}

export function setSignedOut(): void {
    publish({status: "signed_out", user: null, token: null});
}
