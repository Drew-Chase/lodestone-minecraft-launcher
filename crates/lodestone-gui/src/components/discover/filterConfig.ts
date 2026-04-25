import type {ContentTypeKey, FilterState, SourceKey} from "../../types/content";

export interface FilterSectionDef {
    key: keyof FilterState;
    label: string;
    items: string[];
    searchable?: boolean;
}

const CATEGORIES = [
    "Adventure", "Magic", "Tech", "Exploration",
    "Survival", "Performance", "Quests", "Multiplayer",
];

const MODRINTH_LOADERS = [
    "Fabric", "Forge", "NeoForge", "Quilt", "LiteLoader",
    "Bukkit", "Spigot", "Paper", "Purpur", "Sponge",
    "BungeeCord", "Velocity", "Waterfall", "Folia",
];

const CURSEFORGE_LOADERS = [
    "Forge", "Fabric", "Quilt", "NeoForge", "LiteLoader", "Cauldron",
];

const SHADER_LOADERS = ["Iris", "OptiFine", "Canvas"];

const ENVIRONMENTS = ["Client", "Server", "Client & Server"];

function getLoaders(source: SourceKey): string[] {
    switch (source) {
        case "modrinth":
            return MODRINTH_LOADERS;
        case "curseforge":
            return CURSEFORGE_LOADERS;
        default:
            return MODRINTH_LOADERS;
    }
}

export function getFilterSections(
    contentType: ContentTypeKey,
    source: SourceKey,
): FilterSectionDef[] {
    const sections: FilterSectionDef[] = [];

    sections.push({key: "categories", label: "Category", items: CATEGORIES});

    switch (contentType) {
        case "mod":
            sections.push({key: "loaders", label: "Loader", items: getLoaders(source)});
            break;
        case "modpack":
            sections.push({key: "loaders", label: "Loader", items: getLoaders(source)});
            break;
        case "shaderpack":
            sections.push({key: "loaders", label: "Shader Loader", items: SHADER_LOADERS});
            break;
        // resourcepack, datapack, world — no loaders
    }

    // Versions section is always present but items come from the hook, not static config.
    // We mark it with searchable + empty items; the drawer fills them dynamically.
    sections.push({key: "versions", label: "Minecraft Version", items: [], searchable: true});

    if (contentType === "mod") {
        sections.push({key: "environment", label: "Environment", items: ENVIRONMENTS});
    }

    return sections;
}

/** Return true if the given filter key is relevant for this content type. */
export function isFilterKeyRelevant(key: keyof FilterState, contentType: ContentTypeKey): boolean {
    if (key === "categories" || key === "versions") return true;
    if (key === "loaders") return ["mod", "modpack", "shaderpack"].includes(contentType);
    if (key === "environment") return contentType === "mod";
    return false;
}
