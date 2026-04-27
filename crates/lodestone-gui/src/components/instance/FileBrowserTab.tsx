import {useCallback, useEffect, useState} from "react";
import {Button} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {I} from "../shell/icons";
import {cardSurfaceStyle, type Instance} from "../library/instances";

type Props = {
    instance: Instance;
};

interface FileEntry {
    name: string;
    isDir: boolean;
    sizeBytes: number;
    lastModified: string;
}

function formatBytes(bytes: number): string {
    if (bytes === 0) return "—";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatDate(iso: string): string {
    if (!iso) return "—";
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, {
        year: "numeric",
        month: "short",
        day: "numeric",
        hour: "2-digit",
        minute: "2-digit",
    });
}

export default function FileBrowserTab({instance}: Props) {
    const [currentPath, setCurrentPath] = useState("");
    const [entries, setEntries] = useState<FileEntry[]>([]);

    const fetchEntries = useCallback(async () => {
        try {
            const result = await invoke<FileEntry[]>("list_instance_files", {
                instancePath: instance.instancePath,
                subPath: currentPath,
            });
            setEntries(result);
        } catch {
            setEntries([]);
        }
    }, [instance.instancePath, currentPath]);

    useEffect(() => {
        fetchEntries();
    }, [fetchEntries]);

    const navigateInto = (dirName: string) => {
        setCurrentPath((prev) => (prev ? `${prev}/${dirName}` : dirName));
    };

    const navigateUp = () => {
        setCurrentPath((prev) => {
            const parts = prev.split("/").filter(Boolean);
            parts.pop();
            return parts.join("/");
        });
    };

    const navigateToBreadcrumb = (index: number) => {
        const parts = currentPath.split("/").filter(Boolean);
        setCurrentPath(parts.slice(0, index + 1).join("/"));
    };

    const handleOpenInExplorer = () => {
        const sep = instance.instancePath.includes("/") ? "/" : "\\";
        const fullPath = currentPath
            ? `${instance.instancePath}${sep}${currentPath.replace(/\//g, sep)}`
            : instance.instancePath;
        invoke("open_directory", {path: fullPath});
    };

    const pathParts = currentPath.split("/").filter(Boolean);

    return (
        <div className="flex-1 flex flex-col min-h-0 px-7 pt-5 pb-5">
            {/* Breadcrumb + actions */}
            <div className="flex-shrink-0 flex items-center gap-2 mb-3.5">
                <Button
                    isIconOnly
                    variant="bordered"
                    size="sm"
                    aria-label="Up"
                    className="w-7 h-7 min-w-0"
                    isDisabled={!currentPath}
                    onPress={navigateUp}
                >
                    <I.chevRight size={12} className="rotate-180"/>
                </Button>
                <div className="flex items-center gap-1 text-xs font-mono min-w-0 flex-1 overflow-hidden">
                    <button
                        type="button"
                        onClick={() => setCurrentPath("")}
                        className="text-mc-green hover:underline cursor-pointer flex-shrink-0"
                    >
                        {instance.name}
                    </button>
                    {pathParts.map((part, i) => (
                        <span key={i} className="flex items-center gap-1 flex-shrink-0">
                            <span className="text-ink-4">/</span>
                            <button
                                type="button"
                                onClick={() => navigateToBreadcrumb(i)}
                                className={`cursor-pointer hover:underline ${
                                    i === pathParts.length - 1 ? "text-ink-0" : "text-mc-green"
                                }`}
                            >
                                {part}
                            </button>
                        </span>
                    ))}
                </div>
                <Button
                    variant="bordered"
                    size="sm"
                    startContent={<I.external size={11}/>}
                    onPress={handleOpenInExplorer}
                >
                    Open in Explorer
                </Button>
                <Button
                    isIconOnly
                    variant="bordered"
                    size="sm"
                    aria-label="Refresh"
                    onPress={fetchEntries}
                >
                    <I.refresh size={13}/>
                </Button>
            </div>

            {/* File list */}
            <div
                className="flex-1 min-h-0 overflow-y-auto rounded-md border border-line"
                style={cardSurfaceStyle}
            >
                {/* Table header */}
                <div className="flex items-center px-3.5 py-2 text-[0.625rem] font-semibold tracking-[0.06em] uppercase text-ink-3 border-b border-line sticky top-0 bg-bg-1 z-10">
                    <div className="flex-1">Name</div>
                    <div className="w-[100px] text-right">Size</div>
                    <div className="w-[160px] text-right">Modified</div>
                </div>

                {entries.length === 0 ? (
                    <div className="py-8 text-center text-ink-3 text-xs">
                        Empty directory
                    </div>
                ) : (
                    entries.map((entry) => (
                        <div
                            key={entry.name}
                            className={`flex items-center px-3.5 py-2.5 border-b border-[color-mix(in_oklab,var(--line)_60%,transparent)] text-xs hover:bg-[rgba(255,255,255,0.02)] ${
                                entry.isDir ? "cursor-pointer" : ""
                            }`}
                            onClick={() => entry.isDir && navigateInto(entry.name)}
                        >
                            <div className="flex items-center gap-2 flex-1 min-w-0">
                                {entry.isDir ? (
                                    <I.folder size={14} className="text-amber-400 flex-shrink-0"/>
                                ) : (
                                    <I.hardDrive size={14} className="text-ink-3 flex-shrink-0"/>
                                )}
                                <span className={`truncate ${entry.isDir ? "font-semibold" : "text-ink-1"}`}>
                                    {entry.name}
                                </span>
                            </div>
                            <div className="w-[100px] text-right font-mono text-ink-3">
                                {entry.isDir ? "—" : formatBytes(entry.sizeBytes)}
                            </div>
                            <div className="w-[160px] text-right text-ink-3">
                                {formatDate(entry.lastModified)}
                            </div>
                        </div>
                    ))
                )}
            </div>
        </div>
    );
}
