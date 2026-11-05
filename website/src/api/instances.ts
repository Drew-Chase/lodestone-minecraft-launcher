import {apiRequest} from "./client.ts";

// Shapes from the instance, version and share endpoints. Keep these in step with the API.

export type Role = "owner" | "editor" | "viewer";
export type Access = "viewer" | "editor";

export interface Version {
    id: number;
    created_at: string;
    file_count: number;
    total_bytes: number;
    pushed_by: { username: string | null };
}

export interface Instance {
    id: number;
    name: string;
    role: Role;
    owner: { username: string | null };
    latest_version: Version | null;
    created_at: string;
}

export interface FileEntry {
    /** Relative path, with "/" between folders. */
    path: string;
    sha256: string;
    size: number;
    mtime_ms: number;
    /** A presigned GET that lasts 15 minutes. Do not send the access token to it. */
    url: string;
}

export interface VersionDetail {
    version: Version;
    files: FileEntry[];
}

export interface Share {
    username: string;
    access: Access;
    granted_at: string;
}

export async function listInstances(): Promise<Instance[]> {
    const data = await apiRequest<{ instances: Instance[] }>("/api/instances", {auth: true});
    return data.instances;
}

export async function createInstance(name: string): Promise<Instance> {
    const data = await apiRequest<{ instance: Instance }>("/api/instances", {
        method: "POST",
        body: {name},
        auth: true,
    });
    return data.instance;
}

export async function getInstance(id: number): Promise<Instance> {
    const data = await apiRequest<{ instance: Instance }>(`/api/instances/${id}`, {auth: true});
    return data.instance;
}

/** Removes an instance and its versions. Owners only. */
export function deleteInstance(id: number): Promise<void> {
    return apiRequest<void>(`/api/instances/${id}`, {method: "DELETE", auth: true});
}

/** Versions, newest first. */
export async function listVersions(id: number): Promise<Version[]> {
    const data = await apiRequest<{ versions: Version[] }>(`/api/instances/${id}/versions`, {auth: true});
    return data.versions;
}

export function getVersion(id: number, versionId: number): Promise<VersionDetail> {
    return apiRequest<VersionDetail>(`/api/instances/${id}/versions/${versionId}`, {auth: true});
}

/** Who this instance is shared with. Owners only. */
export async function listShares(id: number): Promise<Share[]> {
    const data = await apiRequest<{ shares: Share[] }>(`/api/instances/${id}/shares`, {auth: true});
    return data.shares;
}

/** Shares with a user, or changes their access when they already have a share. */
export function shareInstance(id: number, username: string, access: Access): Promise<{ share: Share }> {
    return apiRequest<{ share: Share }>(`/api/instances/${id}/shares`, {
        method: "POST",
        body: {username, access},
        auth: true,
    });
}

export function removeShare(id: number, username: string): Promise<void> {
    return apiRequest<void>(`/api/instances/${id}/shares/${encodeURIComponent(username)}`, {
        method: "DELETE",
        auth: true,
    });
}
