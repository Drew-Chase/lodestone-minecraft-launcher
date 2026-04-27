import {useCallback, useEffect, useState} from "react";
import {Button} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import Scene from "../shell/Scene";
import {I} from "../shell/icons";
import {cardSurfaceStyle, type Instance} from "../library/instances";
import type {Biome} from "../shell/Scene";

type Props = {
    instance: Instance;
};

interface WorldEntry {
    dirName: string;
    worldName: string;
    seed: number;
    gameMode: number;
    hardcore: boolean;
    difficulty: number;
    playtimeTicks: number;
    minecraftVersion: string;
    sizeBytes: number;
    lastModified: string;
    hasIcon: boolean;
}

const biomes: Biome[] = ["forest", "desert", "ocean", "nether", "end", "cherry", "snow", "mushroom"];

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

function formatPlaytime(ticks: number): string {
    const totalSeconds = Math.floor(ticks / 20);
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    if (hours === 0) return `${minutes}m`;
    return `${hours}h`;
}

function gameModeName(mode: number, hardcore: boolean): string {
    if (hardcore) return "Hardcore";
    switch (mode) {
        case 0: return "Survival";
        case 1: return "Creative";
        case 2: return "Adventure";
        case 3: return "Spectator";
        default: return "Survival";
    }
}

function gameModeColor(mode: number, hardcore: boolean): string {
    if (hardcore) return "#ff5e5e";
    switch (mode) {
        case 1: return "var(--cyan)";
        case 2: return "var(--violet)";
        default: return "var(--mc-green)";
    }
}

function difficultyName(diff: number): string {
    switch (diff) {
        case 0: return "Peaceful";
        case 1: return "Easy";
        case 2: return "Normal";
        case 3: return "Hard";
        default: return "Normal";
    }
}

function seedToBiome(seed: number): Biome {
    return biomes[Math.abs(seed) % biomes.length];
}

export default function WorldsTab({instance}: Props) {
    const [worlds, setWorlds] = useState<WorldEntry[]>([]);
    const [search, setSearch] = useState("");

    const fetchWorlds = useCallback(async () => {
        try {
            const result = await invoke<WorldEntry[]>("list_instance_worlds", {
                instancePath: instance.instancePath,
            });
            setWorlds(result);
        } catch {
            setWorlds([]);
        }
    }, [instance.instancePath]);

    useEffect(() => {
        fetchWorlds();
    }, [fetchWorlds]);

    const handleDelete = async (dirName: string, worldName: string) => {
        if (!confirm(`Delete world "${worldName || dirName}"? This cannot be undone.`)) return;
        await invoke("delete_world", {instancePath: instance.instancePath, dirName});
        fetchWorlds();
    };

    const handleOpenFolder = (dirName: string) => {
        const sep = instance.instancePath.includes("/") ? "/" : "\\";
        invoke("open_directory", {path: `${instance.instancePath}${sep}saves${sep}${dirName}`});
    };

    const handleCopySeed = (seed: number) => {
        navigator.clipboard.writeText(String(seed));
    };

    const filtered = worlds.filter((w) =>
        (w.worldName || w.dirName).toLowerCase().includes(search.toLowerCase()),
    );

    return (
        <div className="flex-1 flex flex-col min-h-0 px-7 pt-5 pb-5">
            {/* Toolbar */}
            <div className="flex-shrink-0 flex items-center gap-2.5 mb-3.5">
                <div className="relative flex-1 max-w-[280px]">
                    <span className="absolute left-2.5 top-1/2 -translate-y-1/2 text-ink-3">
                        <I.search size={12}/>
                    </span>
                    <input
                        placeholder="Search worlds…"
                        value={search}
                        onChange={(e) => setSearch(e.target.value)}
                        className="w-full bg-bg-1 border border-line rounded-lg py-[7px] pl-7 pr-2.5 text-ink-0 text-xs outline-none font-sans"
                    />
                </div>
                <div className="flex-1"/>
                <Button variant="bordered" size="sm" className="text-[0.6875rem]" startContent={<I.refresh size={11}/>} onPress={fetchWorlds}>
                    Refresh
                </Button>
            </div>

            {worlds.length === 0 ? (
                <div className="flex-1 flex items-center justify-center text-ink-3 text-sm">
                    <div className="text-center">
                        <I.globe size={32} className="mx-auto mb-3 opacity-40"/>
                        <div>No worlds found</div>
                        <div className="text-xs mt-1 text-ink-4">Worlds will appear here after you play</div>
                    </div>
                </div>
            ) : (
                <div className="flex-1 min-h-0 overflow-y-auto -mx-1 px-1 pb-1">
                    <div className="grid grid-cols-3 gap-3.5">
                        {filtered.map((w) => (
                            <WorldCard
                                key={w.dirName}
                                world={w}
                                instancePath={instance.instancePath}
                                instanceName={instance.name}
                                onOpenFolder={() => handleOpenFolder(w.dirName)}
                                onDelete={() => handleDelete(w.dirName, w.worldName)}
                                onCopySeed={() => handleCopySeed(w.seed)}
                            />
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}

function WorldCard({world: w, instancePath, instanceName, onOpenFolder, onDelete, onCopySeed}: {
    world: WorldEntry;
    instancePath: string;
    instanceName: string;
    onOpenFolder: () => void;
    onDelete: () => void;
    onCopySeed: () => void;
}) {
    const [iconUrl, setIconUrl] = useState<string | null>(null);
    const name = w.worldName || w.dirName;
    const mode = gameModeName(w.gameMode, w.hardcore);
    const modeColor = gameModeColor(w.gameMode, w.hardcore);
    const diff = difficultyName(w.difficulty);
    const biome = seedToBiome(w.seed);
    const seedNum = Math.abs(w.seed % 100);

    // Load icon.png if available
    useEffect(() => {
        if (!w.hasIcon) return;
        invoke<number[]>("read_world_icon", {
            instancePath,
            dirName: w.dirName,
        }).then((bytes) => {
            const uint8 = new Uint8Array(bytes);
            const blob = new Blob([uint8], {type: "image/png"});
            setIconUrl(URL.createObjectURL(blob));
        }).catch(() => {});
        return () => {
            if (iconUrl) URL.revokeObjectURL(iconUrl);
        };
    }, [w.hasIcon, w.dirName, instancePath]); // eslint-disable-line react-hooks/exhaustive-deps

    return (
        <div className="rounded-xl overflow-hidden border border-line cursor-pointer" style={cardSurfaceStyle}>
            {/* Thumbnail — 150px, icon.png or Scene fallback */}
            <div className="relative h-[150px]">
                {iconUrl ? (
                    <img src={iconUrl} alt={name} className="w-full h-full object-cover"/>
                ) : (
                    <Scene biome={biome} seed={seedNum}/>
                )}
                <div className="absolute inset-0" style={{background: "linear-gradient(180deg, transparent 50%, rgba(0,0,0,0.8) 100%)"}}/>
                {/* Title overlay at bottom */}
                <div className="absolute bottom-2.5 left-3 right-3 z-[2]">
                    <div className="text-[15px] font-extrabold -tracking-[0.3px] mb-0.5">{name}</div>
                    <div className="text-[10px] text-ink-2 font-mono">
                        {instanceName} · {w.minecraftVersion || "Unknown"}
                    </div>
                </div>
            </div>

            {/* Stat row: mode · difficulty + playtime + size */}
            <div className="px-3.5 py-2.5 flex items-center gap-2.5 text-[11px] text-ink-3 font-mono">
                <span className="inline-flex items-center gap-1.5 font-bold tracking-[0.3px]">
                    <span className="w-1.5 h-1.5 rounded-full flex-shrink-0" style={{background: modeColor, boxShadow: `0 0 6px ${modeColor}`}}/>
                    <span className="text-ink-2">{mode.toUpperCase()}</span>
                    <span className="text-ink-4">·</span>
                    <span className="text-ink-3">{diff.toUpperCase()}</span>
                </span>
                <div className="flex-1"/>
                <span className="flex items-center gap-1"><I.clock size={10}/> {formatPlaytime(w.playtimeTicks)}</span>
                <span className="flex items-center gap-1"><I.hardDrive size={10}/> {formatBytes(w.sizeBytes)}</span>
            </div>

            {/* Seed pill + actions */}
            <div className="px-3.5 pb-3 flex items-center gap-1.5">
                <div
                    className="flex-1 min-w-0 px-2.5 py-1.5 rounded-md text-[10px] text-ink-3 font-mono overflow-hidden text-ellipsis whitespace-nowrap"
                    style={{background: "rgba(255,255,255,0.03)", border: "1px solid var(--line)"}}
                >
                    SEED · {String(w.seed)}
                </div>
                <Button isIconOnly variant="bordered" size="sm" aria-label="Copy seed" className="w-[30px] h-[30px] min-w-0" onPress={onCopySeed}>
                    <I.copy size={13}/>
                </Button>
                <Button isIconOnly variant="bordered" size="sm" aria-label="Open folder" className="w-[30px] h-[30px] min-w-0" onPress={onOpenFolder}>
                    <I.folder size={13}/>
                </Button>
                <Button isIconOnly variant="bordered" size="sm" aria-label="Delete" className="w-[30px] h-[30px] min-w-0" onPress={onDelete}>
                    <I.trash size={13}/>
                </Button>
            </div>
        </div>
    );
}
