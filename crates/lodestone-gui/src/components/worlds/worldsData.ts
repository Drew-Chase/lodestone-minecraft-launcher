import type {Biome} from "../shell/Scene";

export type Gamemode = "Survival" | "Creative" | "Hardcore";
export type Difficulty = "Peaceful" | "Easy" | "Normal" | "Hard";
export type WorldViewMode = "grid" | "table";

export type World = {
    name: string;
    inst: string;
    biome: Biome;
    seed: number;
    gamemode: Gamemode;
    difficulty: Difficulty;
    size: string;
    played: string;
    last: string;
    seedStr: string;
    version: string;
    backed?: boolean;
    pinned?: boolean;
};

export const worlds: World[] = [
    {
        name: "Aether Kingdom", inst: "Aether Legacy", biome: "cherry", seed: 3,
        gamemode: "Survival", difficulty: "Hard", size: "412 MB", played: "84h",
        last: "2h ago", seedStr: "-8429174632", version: "1.20.1",
        backed: true, pinned: true,
    },
    {
        name: "Skyfall", inst: "All the Mods 9", biome: "end", seed: 7,
        gamemode: "Creative", difficulty: "Peaceful", size: "1.2 GB", played: "23h",
        last: "Yesterday", seedStr: "1993", version: "1.20.1",
        backed: true,
    },
    {
        name: "Redstone Lab", inst: "Vanilla Tweaked", biome: "desert", seed: 11,
        gamemode: "Creative", difficulty: "Peaceful", size: "86 MB", played: "7h",
        last: "3d ago", seedStr: "42", version: "1.20.4",
    },
    {
        name: "Spruce Village", inst: "Better MC", biome: "snow", seed: 14,
        gamemode: "Survival", difficulty: "Normal", size: "248 MB", played: "61h",
        last: "5d ago", seedStr: "2048", version: "1.20.1",
        backed: true,
    },
    {
        name: "Deepslate Mines", inst: "Vault Hunters 3", biome: "ocean", seed: 19,
        gamemode: "Hardcore", difficulty: "Hard", size: "903 MB", played: "142h",
        last: "1w ago", seedStr: "dwarven", version: "1.20.1",
        backed: true,
    },
    {
        name: "Crimson Expedition", inst: "Prominence II RPG", biome: "nether", seed: 27,
        gamemode: "Survival", difficulty: "Hard", size: "522 MB", played: "38h",
        last: "2w ago", seedStr: "666999", version: "1.20.1",
    },
];
