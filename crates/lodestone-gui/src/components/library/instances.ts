import type {Biome} from "../shell/Scene";

export type ChipColor = "green" | "violet" | "amber" | "cyan" | "pink";
export type Loader = "Fabric" | "Vanilla" | "Forge" | "NeoForge" | "Quilt";
export type ViewMode = "grid" | "compact" | "table";

export type Instance = {
    id: number;
    instancePath: string;
    name: string;
    version: string;
    loader: Loader;
    loaderVersion: string | null;
    javaVersion: string | null;
    createdAt: string;
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
        id: 0, instancePath: "", loaderVersion: null, javaVersion: null, createdAt: "",
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
        id: 0, instancePath: "", loaderVersion: null, javaVersion: null, createdAt: "",
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
        id: 0, instancePath: "", loaderVersion: null, javaVersion: null, createdAt: "",
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
        id: 0, instancePath: "", loaderVersion: null, javaVersion: null, createdAt: "",
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
        id: 0, instancePath: "", loaderVersion: null, javaVersion: null, createdAt: "",
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
        id: 0, instancePath: "", loaderVersion: null, javaVersion: null, createdAt: "",
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

// Map a backend InstanceConfig (from the Tauri `list_instances` command) to the
// frontend `Instance` type used by library components.
const loaderColors: Record<string, ChipColor> = {
    vanilla: "green",
    fabric: "violet",
    forge: "amber",
    neoforge: "cyan",
    quilt: "pink",
};

const loaderLabels: Record<string, Loader> = {
    vanilla: "Vanilla",
    fabric: "Fabric",
    forge: "Forge",
    neoforge: "NeoForge",
    quilt: "Quilt",
};

const biomes: Biome[] = ["forest", "desert", "ocean", "nether", "end", "cherry"];

export function configToInstance(config: {
    id: number;
    name: string;
    minecraft_version: string;
    loader: string;
    loader_version: string | null;
    java_version: string | null;
    created_at: string;
    last_played: string | null;
    instance_path: string;
    mod_count?: number;
}): Instance {
    const loaderLabel = loaderLabels[config.loader] ?? "Vanilla";
    const versionStr =
        config.loader === "vanilla"
            ? `${config.minecraft_version} · Vanilla`
            : `${config.minecraft_version} · ${loaderLabel}`;

    // Deterministic biome/seed from the ID
    const biome = biomes[config.id % biomes.length];
    const seed = (config.id % 8) + 1;

    // Relative time for last_played
    let lastPlayed = "Never";
    if (config.last_played) {
        const diff = Date.now() - new Date(config.last_played).getTime();
        const mins = Math.floor(diff / 60000);
        if (mins < 60) lastPlayed = `${mins}m ago`;
        else if (mins < 1440) lastPlayed = `${Math.floor(mins / 60)}h ago`;
        else lastPlayed = `${Math.floor(mins / 1440)}d ago`;
    }

    return {
        id: config.id,
        instancePath: config.instance_path,
        name: config.name,
        version: versionStr,
        loader: loaderLabel,
        loaderVersion: config.loader_version,
        javaVersion: config.java_version,
        createdAt: config.created_at,
        mc: config.minecraft_version,
        biome,
        seed,
        playtime: "0h",
        lastPlayed,
        mods: config.mod_count ?? 0,
        color: loaderColors[config.loader] ?? "green",
    };
}

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
