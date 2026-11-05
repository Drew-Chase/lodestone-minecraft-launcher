import {apiRequest} from "./client.ts";

// Shapes from the friend endpoints. Keep these in step with the API.

/** A person in a friends list. `username` is null for an account that has not chosen one. */
export interface FriendEntry {
    /** The friendship id. */
    id: number;
    username: string | null;
}

export interface FriendLists {
    friends: FriendEntry[];
    incoming: FriendEntry[];
    outgoing: FriendEntry[];
}

export interface FriendRequest {
    id: number;
    status: "pending";
    username: string | null;
}

/** 201 with a pending request, or 200 with an accepted friendship when they had already asked you. */
export type SendRequestResult =
    | { request: FriendRequest }
    | { status: "accepted"; friend: { username: string | null } };

export interface Invite {
    url: string;
    expires_at: string;
}

export interface InviteAccepted {
    status: "accepted";
    friend: { username: string | null };
}

export async function getFriendCode(): Promise<string> {
    const data = await apiRequest<{ code: string }>("/api/friends/code", {auth: true});
    return data.code;
}

export async function resetFriendCode(): Promise<string> {
    const data = await apiRequest<{ code: string }>("/api/friends/code/reset", {method: "POST", auth: true});
    return data.code;
}

/** Finds an account by username. Throws `not_found` when there is none. */
export async function searchUser(username: string): Promise<string | null> {
    const data = await apiRequest<{ user: { username: string | null } }>(
        `/api/friends/search?username=${encodeURIComponent(username)}`,
        {auth: true},
    );
    return data.user.username;
}

export function requestFriendByUsername(username: string): Promise<SendRequestResult> {
    return apiRequest<SendRequestResult>("/api/friends/requests", {
        method: "POST",
        body: {username},
        auth: true,
    });
}

export function requestFriendByCode(code: string): Promise<SendRequestResult> {
    return apiRequest<SendRequestResult>("/api/friends/requests", {
        method: "POST",
        body: {code},
        auth: true,
    });
}

export async function listFriends(): Promise<FriendLists> {
    return apiRequest<FriendLists>("/api/friends", {auth: true});
}

export async function acceptFriendRequest(id: number): Promise<void> {
    await apiRequest<{ status: string }>(`/api/friends/requests/${id}/accept`, {
        method: "POST",
        auth: true,
    });
}

/** Removes a friend, cancels an outgoing request, or declines an incoming one. */
export function removeFriendship(id: number): Promise<void> {
    return apiRequest<void>(`/api/friends/${id}`, {method: "DELETE", auth: true});
}

export function createInvite(): Promise<Invite> {
    return apiRequest<Invite>("/api/friends/invites", {method: "POST", auth: true});
}

export function acceptInvite(token: string): Promise<InviteAccepted> {
    return apiRequest<InviteAccepted>("/api/friends/invites/accept", {
        method: "POST",
        body: {token},
        auth: true,
    });
}
