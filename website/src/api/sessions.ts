// The account's sessions. Each one is a refresh-token family. See api/README.md, "Sessions".
import {apiRequest} from "./client.ts";

export type Session = {
    id: string;
    client: string;
    created_at: string;
    last_used_at: string;
    current: boolean;
};

export type SessionList = {
    sessions: Session[];
};

export function listSessions(): Promise<SessionList> {
    return apiRequest<SessionList>("/api/account/sessions", {auth: true});
}

export function revokeSession(id: string): Promise<void> {
    return apiRequest<void>(`/api/account/sessions/${encodeURIComponent(id)}`, {method: "DELETE", auth: true});
}

export function revokeOtherSessions(): Promise<{status: string; count: number}> {
    return apiRequest<{status: string; count: number}>("/api/account/sessions/revoke-others", {
        method: "POST",
        auth: true,
    });
}
