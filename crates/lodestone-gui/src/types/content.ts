/**
 * Mirrors hopper-mc's Rust model structs for frontend consumption.
 *
 * IMPORTANT: hopper-mc uses `#[serde(flatten)]` on ContentBase, which
 * means all ContentBase fields are serialized at the TOP LEVEL of each
 * item — not nested under a `base` key. The TS interfaces here use
 * `extends ContentBase` to match that flat shape.
 */

export interface Author {
    name: string;
    url: string | null;
}

export interface Links {
    website: string | null;
    source: string | null;
    issues: string | null;
    wiki: string | null;
    discord: string | null;
    donation: string[];
}

export interface License {
    id: string;
    name: string | null;
    url: string | null;
}

export type PlatformId = "Modrinth" | "CurseForge" | "AtLauncher" | "Technic" | "Ftb";

export interface ContentBase {
    id: string;
    slug: string;
    platform: PlatformId;
    title: string;
    summary: string;
    description: string | null;
    authors: Author[];
    icon_url: string | null;
    gallery: string[];
    links: Links;
    downloads: number;
    follows: number;
    license: License | null;
    created: string;
    updated: string;
    categories: string[];
    game_versions: string[];
}

export type SideSupport = "Required" | "Optional" | "Unsupported" | "Unknown";

export interface Dependency {
    project_id: string | null;
    version_id: string | null;
    kind: "Required" | "Optional" | "Incompatible" | "Embedded";
}

export interface ModItem extends ContentBase {
    loaders: string[];
    client_side: SideSupport;
    server_side: SideSupport;
    dependencies: Dependency[];
}

export interface PackItem extends ContentBase {
    loaders: string[];
    mc_version: string | null;
    included_mods_count: number | null;
    has_server_pack: boolean;
}

export interface DatapackItem extends ContentBase {
    pack_format: number | null;
}

export interface ResourcePackItem extends ContentBase {
    pack_format: number | null;
    resolution: number | null;
}

export interface ShaderPackItem extends ContentBase {
    shader_loaders: string[];
}

export interface WorldItem extends ContentBase {
    mc_version: string | null;
    size_bytes: number | null;
}

export type ContentItem =
    | ModItem
    | PackItem
    | DatapackItem
    | ResourcePackItem
    | ShaderPackItem
    | WorldItem;

export type ContentTypeKey =
    | "mod"
    | "modpack"
    | "datapack"
    | "resourcepack"
    | "shaderpack"
    | "world";

export type SortKey = "relevance" | "downloads" | "follows" | "latest" | "updated";

export type SourceKey = "modrinth" | "curseforge" | "atlauncher" | "ftb" | "technic";

export type ViewMode = "grid" | "compact" | "table";

export interface FilterState {
    categories: string[];
    loaders: string[];
    versions: string[];
    environment: string[];
}

export const defaultFilterState: FilterState = {
    categories: [],
    loaders: [],
    versions: [],
    environment: [],
};

export type VersionType = "Release" | "Beta" | "Alpha";

export interface ProjectVersion {
    id: string;
    project_id: string;
    name: string;
    version_number: string;
    changelog: string | null;
    date_published: string;
    downloads: number;
    version_type: VersionType;
    game_versions: string[];
    loaders: string[];
    files: VersionFile[];
    dependencies: Dependency[];
    featured: boolean;
    platform: PlatformId;
}

export interface VersionFile {
    url: string | null;
    filename: string;
    size: number;
    primary: boolean;
    hashes: Record<string, string>;
}

// ---------------------------------------------------------------------------
// Modpack import types
// ---------------------------------------------------------------------------

export interface RecentImport {
    id: string;
    name: string;
    source: string;
    sizeBytes: number;
    importedAt: string;
    fileName: string;
}

export type ModpackProgressStage =
    | "parsing"
    | "creatingInstance"
    | "resolvingFiles"
    | "downloadingMods"
    | "extractingOverrides"
    | "complete"
    | "failed";

export interface ModpackProgress {
    stage: ModpackProgressStage;
    name?: string;
    current?: number;
    total?: number;
    fileName?: string;
    instanceId?: number;
    error?: string;
}

/** Format a file size in bytes to a human-readable string. */
export function formatSize(bytes: number): string {
    if (bytes >= 1_073_741_824) return `${(bytes / 1_073_741_824).toFixed(1)} GB`;
    if (bytes >= 1_048_576) return `${(bytes / 1_048_576).toFixed(1)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${bytes} B`;
}

/** Format a large number with K/M suffix. */
export function formatCount(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return n.toString();
}

/** Relative time string from an ISO-8601 date. */
export function timeAgo(iso: string): string {
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60_000);
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.floor(hrs / 24);
    if (days < 30) return `${days}d ago`;
    const months = Math.floor(days / 30);
    return `${months}mo ago`;
}
