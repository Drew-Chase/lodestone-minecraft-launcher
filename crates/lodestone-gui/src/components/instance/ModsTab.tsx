import {useCallback, useEffect, useState} from "react";
import {Button, Input} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {Switch} from "../Switch";
import {I} from "../shell/icons";
import {cardSurfaceStyle, type Instance} from "../library/instances";

type Props = {
    instance: Instance;
};

interface ModEntry {
    fileName: string;
    fileSizeBytes: number;
    enabled: boolean;
    lastModified: string;
}

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function displayName(fileName: string): string {
    return fileName
        .replace(/\.jar\.disabled$/, "")
        .replace(/\.jar$/, "");
}

export default function ModsTab({instance}: Props) {
    const [mods, setMods] = useState<ModEntry[]>([]);
    const [filter, setFilter] = useState("");

    const fetchMods = useCallback(async () => {
        try {
            const result = await invoke<ModEntry[]>("list_instance_mods", {
                instancePath: instance.instancePath,
            });
            setMods(result);
        } catch {
            setMods([]);
        }
    }, [instance.instancePath]);

    useEffect(() => {
        fetchMods();
    }, [fetchMods]);

    const handleToggle = async (mod: ModEntry, enabled: boolean) => {
        await invoke("toggle_mod", {
            instancePath: instance.instancePath,
            fileName: mod.fileName,
            enabled,
        });
        fetchMods();
    };

    const handleDelete = async (mod: ModEntry) => {
        await invoke("delete_mod", {
            instancePath: instance.instancePath,
            fileName: mod.fileName,
        });
        fetchMods();
    };

    const handleOpenFolder = () => {
        invoke("open_directory", {
            path: `${instance.instancePath}${instance.instancePath.includes("/") ? "/" : "\\"}mods`,
        });
    };

    const filtered = mods.filter((m) =>
        displayName(m.fileName).toLowerCase().includes(filter.toLowerCase()),
    );

    return (
        <div className="flex-1 flex flex-col min-h-0 px-7 pt-5 pb-5">
            {/* Filter row */}
            <div className="flex-shrink-0 flex gap-2.5 mb-3.5">
                <Input
                    placeholder="Filter mods…"
                    size="sm"
                    value={filter}
                    onValueChange={setFilter}
                    classNames={{
                        base: "flex-1",
                        inputWrapper: "bg-[rgba(255,255,255,0.04)] border border-line",
                    }}
                    startContent={<I.search size={14}/>}
                />
                <Button
                    variant="bordered"
                    size="sm"
                    startContent={<I.folder size={13}/>}
                    onPress={handleOpenFolder}
                >
                    Open Folder
                </Button>
                <Button
                    variant="bordered"
                    size="sm"
                    startContent={<I.refresh size={13}/>}
                    onPress={fetchMods}
                >
                    Refresh
                </Button>
            </div>

            {mods.length === 0 ? (
                <div className="flex-1 flex items-center justify-center text-ink-3 text-sm">
                    <div className="text-center">
                        <I.box size={32} className="mx-auto mb-3 opacity-40"/>
                        <div>No mods installed</div>
                        <div className="text-xs mt-1 text-ink-4">
                            Add .jar files to the mods folder to get started
                        </div>
                    </div>
                </div>
            ) : (
                <div
                    className="flex-1 min-h-0 overflow-y-auto rounded-md border border-line"
                    style={cardSurfaceStyle}
                >
                    {filtered.map((m, i) => {
                        const isLast = i === filtered.length - 1;
                        return (
                            <div
                                key={m.fileName}
                                className={`flex items-center gap-3.5 px-4 py-3 transition-colors hover:bg-[rgba(255,255,255,0.02)] ${
                                    isLast ? "" : "border-b border-line"
                                }`}
                            >
                                <div className="flex-shrink-0 w-8 h-8 rounded-lg flex items-center justify-center border border-line bg-[rgba(255,255,255,0.04)] text-ink-2">
                                    <I.box size={14}/>
                                </div>
                                <div className="min-w-0 flex-1">
                                    <div className="text-[0.8125rem] font-semibold truncate">
                                        {displayName(m.fileName)}
                                    </div>
                                    <div className="text-[0.6875rem] text-ink-3 mt-0.5 font-mono">
                                        {formatBytes(m.fileSizeBytes)}
                                    </div>
                                </div>
                                <Switch
                                    isSelected={m.enabled}
                                    onValueChange={(v) => handleToggle(m, v)}
                                    size="sm"
                                    color="success"
                                    aria-label={`Toggle ${displayName(m.fileName)}`}
                                />
                                <Button
                                    isIconOnly
                                    variant="bordered"
                                    size="sm"
                                    aria-label="Delete"
                                    className="w-7 h-7 min-w-0"
                                    onPress={() => handleDelete(m)}
                                >
                                    <I.trash size={13}/>
                                </Button>
                            </div>
                        );
                    })}
                </div>
            )}

            {mods.length > 0 && (
                <div className="flex-shrink-0 text-xs text-ink-3 mt-2.5 font-mono">
                    {mods.filter((m) => m.enabled).length} enabled · {mods.filter((m) => !m.enabled).length} disabled · {mods.length} total
                </div>
            )}
        </div>
    );
}
