import React from "react";
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

// Shared "glassy card" surface gradient — layered linear-gradient w/ rgba stops
// doesn't cleanly reduce to a single Tailwind utility, so it stays inline.
export const cardSurfaceStyle: React.CSSProperties = {
    background:
        "linear-gradient(180deg, rgba(255,255,255,0.035) 0%, rgba(255,255,255,0.015) 100%)",
};

// Design's .card:hover effect: subtle lift, green-tinted border, drop shadow with
// a faint green rim. Applied to all pressable instance/action cards.
export const cardHoverClass =
    "transition-all duration-200 hover:-translate-y-0.5 hover:border-[rgba(34,255,132,0.25)] hover:shadow-[0_20px_48px_-24px_rgba(0,0,0,0.6),0_0_0_1px_rgba(34,255,132,0.15)]";
