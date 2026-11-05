import {apiRequest} from "./client.ts";

// Shapes from the Minecraft account endpoints. Keep these in step with the API.

export interface MinecraftAccount {
    uuid: string;
    name: string;
    linked_at: string;
}

export interface LinkStart {
    id: string;
    user_code: string;
    verification_uri: string;
    expires_at: string;
    /** Seconds between status checks. */
    interval: number;
}

export type LinkStatusValue = "pending" | "linked" | "denied" | "expired" | "failed";

export type LinkError =
    | "minecraft_not_owned"
    | "minecraft_restricted"
    | "minecraft_linked_elsewhere"
    | "minecraft_unavailable";

export interface LinkStatus {
    status: LinkStatusValue;
    /** Present while pending. A new value replaces the interval for the next check. */
    interval?: number;
    account?: MinecraftAccount;
    error?: LinkError;
}

export async function listMinecraftAccounts(): Promise<MinecraftAccount[]> {
    const data = await apiRequest<{ accounts: MinecraftAccount[] }>("/api/minecraft/accounts", {auth: true});
    return data.accounts;
}

/** Starts linking a Microsoft account to Minecraft. The user enters `user_code` on `verification_uri`. */
export function startMinecraftLink(): Promise<LinkStart> {
    return apiRequest<LinkStart>("/api/minecraft/link", {method: "POST", auth: true});
}

/** Checks a link. A check that comes too soon fails with `slow_down` and a Retry-After header. */
export function getMinecraftLinkStatus(id: string): Promise<LinkStatus> {
    return apiRequest<LinkStatus>(`/api/minecraft/link/${encodeURIComponent(id)}`, {auth: true});
}

export function unlinkMinecraftAccount(uuid: string): Promise<void> {
    return apiRequest<void>(`/api/minecraft/accounts/${encodeURIComponent(uuid)}`, {method: "DELETE", auth: true});
}
