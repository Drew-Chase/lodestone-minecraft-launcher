import {useState} from "react";
import {Button} from "@heroui/react";
import Scene from "../shell/Scene";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {cardSurfaceStyle} from "../surfaces";
import type {Biome} from "../shell/Scene";
import type {ChipColor} from "../library/instances";

type Mode = "Survival" | "Creative" | "Hardcore" | "Adventure";

type World = {
    name: string;
    biome: Biome;
    seed: string;
    mode: Mode;
    diff: string;
    size: string;
    played: string;
    last: string;
    badges: ("pinned" | "cloud" | "backed-up")[];
    version: string;
};

// Design-matched sample data — the v2 instance page shows these six worlds for
// the currently-selected instance.
const worlds: World[] = [
    {name: "Aether Hub", biome: "end", seed: "-4827193047281", mode: "Survival", diff: "Hard", size: "1.4 GB", played: "48h 12m", last: "02:14 today", badges: ["pinned", "cloud"], version: "1.20.1"},
    {name: "Creative Sandbox", biome: "cherry", seed: "8821094572", mode: "Creative", diff: "Peaceful", size: "412 MB", played: "9h 04m", last: "Yesterday", badges: ["cloud"], version: "1.20.1"},
    {name: "SkyBlock Classic", biome: "ocean", seed: "12345", mode: "Survival", diff: "Normal", size: "284 MB", played: "22h 47m", last: "3 days ago", badges: ["backed-up"], version: "1.20.1"},
    {name: "Nether Outpost", biome: "nether", seed: "-918273645", mode: "Hardcore", diff: "Hard", size: "2.1 GB", played: "104h 31m", last: "Last week", badges: ["pinned"], version: "1.20.1"},
    {name: "Pixel Town", biome: "forest", seed: "7391047", mode: "Survival", diff: "Normal", size: "896 MB", played: "31h 18m", last: "2 weeks ago", badges: [], version: "1.19.4"},
    {name: "Desert Expedition", biome: "desert", seed: "-2837461", mode: "Adventure", diff: "Hard", size: "644 MB", played: "14h 22m", last: "Apr 8, 2026", badges: ["backed-up"], version: "1.20.1"},
];

const modeColor: Record<Mode, ChipColor> = {
    Survival: "green",
    Creative: "cyan",
    Hardcore: "amber",
    Adventure: "violet",
};

type FilterKey = "all" | "single" | "backups";

const filters: {id: FilterKey; label: string}[] = [
    {id: "all", label: "All"},
    {id: "single", label: "Singleplayer"},
    {id: "backups", label: "Backups"},
];

// Per-instance worlds tab — matches the v2 design: search + segmented All /
// Singleplayer / Backups filter + Import + New world buttons above a 2-column
// grid of horizontal world cards (128px biome thumbnail on the left, details
// on the right with mode chip, seed, playtime/size, and Play action).
export default function WorldsTab() {
    const [filter, setFilter] = useState<FilterKey>("all");

    const visible = worlds.filter((w) => {
        if (filter === "backups") return w.badges.includes("backed-up");
        // "all" and "single" both render the full list for this sample dataset
        // (a real multiplayer-aware filter would check a server-joined flag).
        return true;
    });

    return (
        <>
            {/* Toolbar */}
            <div className="flex items-center gap-2.5 mb-3.5">
                <div className="relative flex-1 max-w-[280px]">
          <span className="absolute left-2.5 top-1/2 -translate-y-1/2 text-ink-3">
            <I.search size={12}/>
          </span>
                    <input
                        placeholder="Search worlds…"
                        className="w-full bg-bg-1 border border-line rounded-lg py-[7px] pl-7 pr-2.5 text-ink-0 text-xs outline-none font-sans"
                    />
                </div>

                <div className="flex gap-1 p-[3px] bg-bg-1 border border-line rounded-lg">
                    {filters.map((f) => {
                        const active = filter === f.id;
                        return (
                            <button
                                key={f.id}
                                type="button"
                                onClick={() => setFilter(f.id)}
                                className={[
                                    "px-2.5 py-[5px] text-[0.6875rem] rounded-md cursor-pointer transition-colors",
                                    active
                                        ? "bg-bg-2 text-ink-0"
                                        : "bg-transparent text-ink-3 hover:text-ink-1",
                                ].join(" ")}
                            >
                                {f.label}
                            </button>
                        );
                    })}
                </div>

                <div className="flex-1"/>

                <Button
                    variant="bordered"
                    size="sm"
                    className="text-[0.6875rem]"
                    startContent={<I.upload size={11}/>}
                >
                    Import
                </Button>
                <Button
                    variant="bordered"
                    size="sm"
                    className="text-[0.6875rem]"
                    startContent={<I.plus size={11}/>}
                >
                    New world
                </Button>
            </div>

            {/* 2-column card grid */}
            <div className="grid grid-cols-2 gap-3">
                {visible.map((w, i) => (
                    <WorldCard key={i} world={w} seedIndex={i}/>
                ))}
            </div>
        </>
    );
}

function WorldCard({world: w, seedIndex}: {world: World; seedIndex: number}) {
    return (
        <div
            className="rounded-lg overflow-hidden border border-line flex relative"
            style={cardSurfaceStyle}
        >
            {/* 128px biome thumbnail on the left */}
            <div className="w-[128px] h-[128px] flex-shrink-0 relative border-r border-line">
                <Scene biome={w.biome} seed={w.seed.length * 3 + seedIndex}/>
                <div
                    className="absolute inset-0"
                    style={{
                        background:
                            "linear-gradient(135deg, transparent 55%, rgba(8,9,10,0.65) 100%)",
                    }}
                />
                {w.badges.includes("pinned") && (
                    <div className="absolute top-1.5 left-1.5 w-5 h-5 rounded-md flex items-center justify-center text-mc-green backdrop-blur-md bg-[rgba(8,9,10,0.7)]">
                        <I.pin size={10}/>
                    </div>
                )}
                <div className="absolute bottom-1.5 right-1.5 font-mono bg-[rgba(8,9,10,0.6)] rounded-sm px-1.5 py-px text-[0.5625rem] tracking-[0.3px] text-[rgba(255,255,255,0.85)]">
                    {w.version}
                </div>
            </div>

            {/* Right details column */}
            <div className="flex-1 px-3.5 py-3 min-w-0 flex flex-col">
                <div className="flex items-center gap-1.5 mb-1">
                    <div className="text-sm font-semibold -tracking-[0.2px] flex-1 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap">
                        {w.name}
                    </div>
                    {w.badges.includes("cloud") && (
                        <I.cloud size={11} className="text-accent-cyan flex-shrink-0"/>
                    )}
                    {w.badges.includes("backed-up") && (
                        <I.shield size={11} className="text-ink-3 flex-shrink-0"/>
                    )}
                </div>

                <div className="flex gap-1 mb-2 flex-wrap">
                    <Chip variant={modeColor[w.mode]} className="text-[0.5625rem]">
                        {w.mode.toUpperCase()}
                    </Chip>
                    <Chip className="text-[0.5625rem]">{w.diff.toUpperCase()}</Chip>
                </div>

                <div className="text-[0.625rem] text-ink-3 font-mono mb-1 flex items-center gap-1.5">
                    <I.hash size={9}/>
                    <span className="overflow-hidden text-ellipsis whitespace-nowrap">
            {w.seed}
          </span>
                </div>

                <div className="flex gap-3 text-[0.625rem] text-ink-2 font-mono mt-auto">
          <span>
            <span className="text-ink-3">play</span> {w.played}
          </span>
                    <span>
            <span className="text-ink-3">size</span> {w.size}
          </span>
                </div>

                <div className="flex items-center gap-1.5 mt-2 pt-2 border-t border-line">
                    <div className="text-[0.625rem] text-ink-3 flex-1">Saved {w.last}</div>
                    <Button
                        isIconOnly
                        variant="bordered"
                        size="sm"
                        aria-label="Open folder"
                        className="w-6 h-6 min-w-0"
                    >
                        <I.folder size={11}/>
                    </Button>
                    <Button
                        isIconOnly
                        variant="bordered"
                        size="sm"
                        aria-label="More"
                        className="w-6 h-6 min-w-0"
                    >
                        <I.more size={11}/>
                    </Button>
                    <Button
                        color="success"
                        size="sm"
                        className="font-bold text-[0.625rem] min-w-0 h-auto px-2.5 py-[5px]"
                        startContent={<I.play size={9}/>}
                    >
                        Play
                    </Button>
                </div>
            </div>
        </div>
    );
}
