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
    sizeBytes: number;
    lastModified: string;
}

const biomes: Biome[] = ["forest", "desert", "ocean", "nether", "end", "cherry", "snow", "mushroom"];

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function relativeTime(iso: string): string {
    if (!iso) return "—";
    const diff = Date.now() - new Date(iso).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "Just now";
    if (mins < 60) return `${mins}m ago`;
    if (mins < 1440) return `${Math.floor(mins / 60)}h ago`;
    return `${Math.floor(mins / 1440)}d ago`;
}

function hashName(name: string): number {
    let h = 0;
    for (let i = 0; i < name.length; i++) {
        h = ((h << 5) - h + name.charCodeAt(i)) | 0;
    }
    return Math.abs(h);
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

    const handleDelete = async (dirName: string) => {
        if (!confirm(`Delete world "${dirName}"? This cannot be undone.`)) return;
        await invoke("delete_world", {
            instancePath: instance.instancePath,
            dirName,
        });
        fetchWorlds();
    };

    const handleOpenFolder = (dirName: string) => {
        const sep = instance.instancePath.includes("/") ? "/" : "\\";
        invoke("open_directory", {
            path: `${instance.instancePath}${sep}saves${sep}${dirName}`,
        });
    };

    const filtered = worlds.filter((w) =>
        w.dirName.toLowerCase().includes(search.toLowerCase()),
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
                <Button
                    variant="bordered"
                    size="sm"
                    className="text-[0.6875rem]"
                    startContent={<I.refresh size={11}/>}
                    onPress={fetchWorlds}
                >
                    Refresh
                </Button>
            </div>

            {worlds.length === 0 ? (
                <div className="flex-1 flex items-center justify-center text-ink-3 text-sm">
                    <div className="text-center">
                        <I.globe size={32} className="mx-auto mb-3 opacity-40"/>
                        <div>No worlds found</div>
                        <div className="text-xs mt-1 text-ink-4">
                            Worlds will appear here after you play
                        </div>
                    </div>
                </div>
            ) : (
                <div className="flex-1 min-h-0 overflow-y-auto -mx-1 px-1 pb-1">
                    <div className="grid grid-cols-2 gap-3">
                        {filtered.map((w) => {
                            const h = hashName(w.dirName);
                            const biome = biomes[h % biomes.length];
                            return (
                                <div
                                    key={w.dirName}
                                    className="rounded-lg overflow-hidden border border-line flex relative"
                                    style={cardSurfaceStyle}
                                >
                                    <div className="w-[128px] h-[128px] flex-shrink-0 relative border-r border-line">
                                        <Scene biome={biome} seed={h % 20}/>
                                        <div
                                            className="absolute inset-0"
                                            style={{
                                                background:
                                                    "linear-gradient(135deg, transparent 55%, rgba(8,9,10,0.65) 100%)",
                                            }}
                                        />
                                    </div>
                                    <div className="flex-1 px-3.5 py-3 min-w-0 flex flex-col">
                                        <div className="text-sm font-semibold -tracking-[0.2px] truncate mb-2">
                                            {w.dirName}
                                        </div>
                                        <div className="flex gap-3 text-[0.625rem] text-ink-2 font-mono mt-auto">
                                            <span>
                                                <span className="text-ink-3">size</span> {formatBytes(w.sizeBytes)}
                                            </span>
                                        </div>
                                        <div className="flex items-center gap-1.5 mt-2 pt-2 border-t border-line">
                                            <div className="text-[0.625rem] text-ink-3 flex-1">
                                                Modified {relativeTime(w.lastModified)}
                                            </div>
                                            <Button
                                                isIconOnly
                                                variant="bordered"
                                                size="sm"
                                                aria-label="Open folder"
                                                className="w-6 h-6 min-w-0"
                                                onPress={() => handleOpenFolder(w.dirName)}
                                            >
                                                <I.folder size={11}/>
                                            </Button>
                                            <Button
                                                isIconOnly
                                                variant="bordered"
                                                size="sm"
                                                aria-label="Delete"
                                                className="w-6 h-6 min-w-0"
                                                onPress={() => handleDelete(w.dirName)}
                                            >
                                                <I.trash size={11}/>
                                            </Button>
                                        </div>
                                    </div>
                                </div>
                            );
                        })}
                    </div>
                </div>
            )}
        </div>
    );
}
