import type {Access, Role} from "../../api/instances.ts";

/** Storage is counted in decimal units: 1 GB is 1,000,000,000 bytes. */
const KB = 1e3;
const MB = 1e6;
const GB = 1e9;

function trimmed(value: number): string {
    return Number(value.toFixed(1)).toLocaleString();
}

/** "1.25 GB" for the storage meter. Two places at most. */
export function formatGigabytes(bytes: number): string {
    return `${Number((bytes / GB).toFixed(2)).toLocaleString()} GB`;
}

/** A file size in the largest decimal unit that keeps the number above one: "512 B", "3.4 MB". */
export function formatSize(bytes: number): string {
    if (bytes < KB) return `${bytes} B`;
    if (bytes < MB) return `${trimmed(bytes / KB)} KB`;
    if (bytes < GB) return `${trimmed(bytes / MB)} MB`;
    return `${trimmed(bytes / GB)} GB`;
}

/** The reader's local date and time for a file's modification time. */
export function formatModified(mtimeMs: number): string {
    const date = new Date(mtimeMs);
    return Number.isNaN(date.getTime()) ? "Unknown" : date.toLocaleString();
}

/** The first twelve hex characters of a SHA-256, enough to tell files apart at a glance. */
export function shortHash(sha256: string): string {
    return sha256.slice(0, 12);
}

export const ROLE_LABEL: Readonly<Record<Role, string>> = {
    owner: "Owner",
    editor: "Editor",
    viewer: "Viewer",
};

export const ACCESS_LABEL: Readonly<Record<Access, string>> = {
    viewer: "Viewer",
    editor: "Editor",
};

export function fileCountLabel(count: number): string {
    return count === 1 ? "1 file" : `${count} files`;
}
