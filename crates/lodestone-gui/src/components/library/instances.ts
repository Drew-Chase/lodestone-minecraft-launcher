import type {Biome} from "../shell/Scene";

export type ChipColor = "green" | "violet" | "amber" | "cyan" | "pink";
export type Loader = "Fabric" | "Vanilla" | "Forge" | "NeoForge" | "Quilt";
export type ViewMode = "grid" | "compact" | "table";

export type Instance = {
    name: string;
    version: string;
    loader: Loader;
    mc: string;
    biome: Biome;
    seed: number;
    playtime: string;
    lastPlayed: string;
    mods: number;
    playing?: boolean;
    color: ChipColor;
};

export const instances: Instance[] = [
    {
        name: "Aether Legacy",
        version: "1.20.1 · Fabric",
        loader: "Fabric",
        mc: "1.20.1",
        biome: "end",
        seed: 3,
        playtime: "24h",
        lastPlayed: "12m ago",
        mods: 87,
        playing: true,
        color: "violet",
    },
    {
        name: "Vanilla Survival",
        version: "1.20.4 · Vanilla",
        loader: "Vanilla",
        mc: "1.20.4",
        biome: "forest",
        seed: 1,
        playtime: "156h",
        lastPlayed: "2d ago",
        mods: 0,
        color: "green",
    },
    {
        name: "Create: Above & Beyond",
        version: "1.16.5 · Forge",
        loader: "Forge",
        mc: "1.16.5",
        biome: "desert",
        seed: 5,
        playtime: "42h",
        lastPlayed: "5d ago",
        mods: 214,
        color: "amber",
    },
    {
        name: "RLCraft Extreme",
        version: "1.12.2 · Forge",
        loader: "Forge",
        mc: "1.12.2",
        biome: "nether",
        seed: 2,
        playtime: "8h",
        lastPlayed: "3w ago",
        mods: 171,
        color: "pink",
    },
    {
        name: "SkyFactory 4",
        version: "1.12.2 · Forge",
        loader: "Forge",
        mc: "1.12.2",
        biome: "ocean",
        seed: 8,
        playtime: "67h",
        lastPlayed: "1w ago",
        mods: 198,
        color: "cyan",
    },
    {
        name: "Cherry Grove Peaceful",
        version: "1.20.4 · Vanilla",
        loader: "Vanilla",
        mc: "1.20.4",
        biome: "cherry",
        seed: 4,
        playtime: "3h",
        lastPlayed: "1mo ago",
        mods: 0,
        color: "pink",
    },
];

// Re-export the shared surface tokens from their new shared home so existing
// import paths keep working without churn.
export {cardSurfaceStyle, cardHoverClass} from "../surfaces";

// URL-friendly slug for a given instance name. Used as the :slug route param
// on the detail page (e.g. "Aether Legacy" → "aether-legacy").
export function toSlug(name: string): string {
    return name
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, "-")
        .replace(/^-+|-+$/g, "");
}

export function findInstanceBySlug(slug: string): Instance | undefined {
    return instances.find((i) => toSlug(i.name) === slug);
}
