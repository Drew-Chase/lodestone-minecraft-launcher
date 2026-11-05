import {apiRequest} from "./client.ts";

// Shapes from the storage endpoint. Keep these in step with the API.

export interface Storage {
    /** Bytes used across the instances this account owns, including the ones shared out. */
    used_bytes: number;
    /** 0 means there is no plan. */
    limit_bytes: number;
    plan: string;
    name: string;
}

export function fetchStorage(): Promise<Storage> {
    return apiRequest<Storage>("/api/storage", {auth: true});
}
