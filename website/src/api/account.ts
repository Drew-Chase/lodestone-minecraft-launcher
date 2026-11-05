import {apiRequest} from "./client.ts";
import type {User} from "./types.ts";

// Shapes from the account endpoints. Keep these in step with the API.

/** The signed-in user from GET /api/me. The session's user object does not carry the username. */
export interface AccountUser extends User {
    username: string | null;
}

export async function fetchMe(): Promise<AccountUser> {
    const data = await apiRequest<{ user: AccountUser }>("/api/me", {auth: true});
    return data.user;
}

/** Sets the username and returns the saved value. */
export async function setUsername(username: string): Promise<string> {
    const data = await apiRequest<{ username: string }>("/api/account/username", {
        method: "POST",
        body: {username},
        auth: true,
    });
    return data.username;
}
