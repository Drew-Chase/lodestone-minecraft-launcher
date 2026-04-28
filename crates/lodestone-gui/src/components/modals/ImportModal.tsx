import {useEffect, useRef, useState, useCallback, DragEvent} from "react";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import ModalShell from "./ModalShell";
import {FooterBtn, Label} from "./primitives";
import {I} from "../shell/icons";
import {RecentImport, formatSize, timeAgo} from "../../types/content";
import type {InstallProgress} from "../../context/LaunchContext";

type Props = {isOpen: boolean; onClose: () => void};

type TabKey = "file" | "url" | "restore";

const tabs: {key: TabKey; label: string}[] = [
    {key: "file", label: "From file"},
    {key: "url", label: "From URL"},
    {key: "restore", label: "Restore backup"},
];

const fileTiles: {ext: string; rotate: number; translateY: number}[] = [
    {ext: ".zip", rotate: -5, translateY: 4},
    {ext: ".mrpack", rotate: 0, translateY: 0},
    {ext: ".mcpack", rotate: 5, translateY: 4},
];

export default function ImportModal({isOpen, onClose}: Props) {
    const [activeTab, setActiveTab] = useState<TabKey>("file");
    const [recents, setRecents] = useState<RecentImport[]>([]);
    const [installing, setInstalling] = useState(false);
    const [progress, setProgress] = useState<InstallProgress | null>(null);
    const [dragOver, setDragOver] = useState(false);
    const fileInputRef = useRef<HTMLInputElement>(null);

    // Load recent imports when modal opens
    useEffect(() => {
        if (!isOpen) return;
        invoke<RecentImport[]>("list_recent_imports")
            .then(setRecents)
            .catch(() => setRecents([]));
    }, [isOpen]);

    // Listen for progress events (uses same install-progress as Downloads popover)
    useEffect(() => {
        if (!installing) return;

        const unlisteners: (() => void)[] = [];

        listen<InstallProgress>("install-progress", (event) => {
            setProgress(event.payload);
        }).then((u) => unlisteners.push(u));

        listen<{instanceId: number; instanceName: string}>("install-completed", () => {
            setInstalling(false);
            // Refresh recents after import
            invoke<RecentImport[]>("list_recent_imports")
                .then(setRecents)
                .catch(() => {});
        }).then((u) => unlisteners.push(u));

        return () => {
            for (const u of unlisteners) u();
        };
    }, [installing]);

    const importFile = useCallback(async (filePath: string) => {
        setInstalling(true);
        setProgress(null);
        try {
            await invoke("import_modpack", {filePath});
        } catch (e) {
            setInstalling(false);
        }
    }, []);

    const handleBrowse = useCallback(() => {
        fileInputRef.current?.click();
    }, []);

    const handleFileChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
        const files = e.target.files;
        if (files && files.length > 0) {
            const file = files[0];
            const path = (file as any).path;
            if (path) {
                importFile(path);
            }
        }
        // Reset input so the same file can be re-selected
        e.target.value = "";
    }, [importFile]);

    const handleDrop = useCallback((e: DragEvent<HTMLDivElement>) => {
        e.preventDefault();
        setDragOver(false);
        const files = e.dataTransfer?.files;
        if (files && files.length > 0) {
            const file = files[0];
            // Tauri provides the file path via webkitRelativePath or the File object
            const path = (file as any).path ?? file.name;
            if (path) {
                importFile(path);
            }
        }
    }, [importFile]);

    const handleReimport = useCallback(async (importId: string) => {
        setInstalling(true);
        setProgress(null);
        try {
            await invoke("reimport_modpack", {importId});
        } catch (e) {
            setInstalling(false);
        }
    }, []);

    const progressPercent = progress
        ? Math.round(progress.progress * 100)
        : null;

    return (
        <ModalShell
            isOpen={isOpen}
            onClose={onClose}
            title="Import Instance"
            subtitle="From a modpack file, CurseForge URL, or backup archive."
            icon={I.download}
            accent="var(--cyan)"
            size="2xl"
            footer={
                <>
                    <FooterBtn onClick={onClose}>Cancel</FooterBtn>
                    <FooterBtn primary accent="var(--cyan)" onClick={handleBrowse} disabled={installing}>
                        {installing ? "Installing..." : "Browse & Import"}
                    </FooterBtn>
                </>
            }
        >
            {/* Hidden file input */}
            <input
                ref={fileInputRef}
                type="file"
                accept=".zip,.mrpack,.mcpack"
                style={{display: "none"}}
                onChange={handleFileChange}
            />

            {/* Progress indicator */}
            {installing && progress && (
                <div className="mb-4 p-3 rounded-lg border border-line" style={{background: "rgba(56,224,255,0.05)"}}>
                    <div className="text-xs font-semibold text-accent-cyan mb-1">
                        {progress.stageLabel}
                    </div>
                    {progress.filesTotal > 0 && (
                        <div className="text-[0.625rem] text-ink-3 mb-1.5">
                            {progress.filesDone} / {progress.filesTotal} files
                        </div>
                    )}
                    <div className="h-1.5 rounded-full bg-[rgba(0,0,0,0.3)] overflow-hidden">
                        <div
                            className="h-full rounded-full transition-all duration-300"
                            style={{
                                width: `${progressPercent}%`,
                                background: "var(--cyan)",
                            }}
                        />
                    </div>
                </div>
            )}

            {/* Segmented tabs */}
            <div className="flex gap-1 p-1 rounded-[10px] bg-[rgba(0,0,0,0.3)] border border-line mb-[18px]">
                {tabs.map((t) => {
                    const active = activeTab === t.key;
                    return (
                        <button
                            key={t.key}
                            type="button"
                            onClick={() => setActiveTab(t.key)}
                            className="flex-1 px-3 py-2 rounded-[7px] text-xs font-semibold cursor-pointer transition-colors"
                            style={{
                                background: active
                                    ? "color-mix(in oklab, var(--cyan) 14%, transparent)"
                                    : "transparent",
                                color: active ? "var(--cyan)" : "var(--ink-2)",
                                border: active
                                    ? "1px solid color-mix(in oklab, var(--cyan) 30%, transparent)"
                                    : "1px solid transparent",
                            }}
                        >
                            {t.label}
                        </button>
                    );
                })}
            </div>

            {/* Drop zone */}
            <div
                className="py-[34px] px-5 rounded-[14px] text-center relative"
                style={{
                    border: `2px dashed color-mix(in oklab, var(--cyan) ${dragOver ? "70%" : "40%"}, transparent)`,
                    background: dragOver
                        ? "radial-gradient(ellipse at center, rgba(56,224,255,0.18), transparent 70%)"
                        : "radial-gradient(ellipse at center, rgba(56,224,255,0.09), transparent 70%)",
                }}
                onDragOver={(e) => {
                    e.preventDefault();
                    setDragOver(true);
                }}
                onDragLeave={() => setDragOver(false)}
                onDrop={handleDrop}
            >
                {/* Splayed file-type tiles */}
                <div className="flex justify-center gap-3 mb-3.5">
                    {fileTiles.map((tile) => (
                        <div
                            key={tile.ext}
                            className="relative w-[60px] h-[70px] rounded-lg border border-line flex flex-col items-center justify-center shadow-[0_6px_20px_rgba(0,0,0,0.4)]"
                            style={{
                                background:
                                    "linear-gradient(180deg, rgba(40,46,42,0.9), rgba(26,30,28,0.9))",
                                transform: `rotate(${tile.rotate}deg) translateY(${tile.translateY}px)`,
                            }}
                        >
                            <div
                                className="font-mono text-accent-cyan font-bold tracking-[0.5px]"
                                style={{fontSize: "0.625rem"}}
                            >
                                {tile.ext}
                            </div>
                            <div
                                className="absolute bottom-2 w-6 h-0.5 rounded-sm"
                                style={{background: "var(--cyan)", opacity: 0.4}}
                            />
                            <div
                                className="absolute bottom-3.5 w-4 h-0.5 rounded-sm"
                                style={{background: "var(--cyan)", opacity: 0.25}}
                            />
                        </div>
                    ))}
                </div>
                <div className="text-sm font-semibold mb-1">Drop file here</div>
                <div className="text-xs text-ink-3">
                    or{" "}
                    <span
                        className="text-accent-cyan underline cursor-pointer"
                        onClick={handleBrowse}
                    >
                        browse your files
                    </span>{" "}
                    · max 4 GB
                </div>
            </div>

            {/* Recent imports */}
            <div className="mt-5">
                <div className="flex items-center justify-between mb-2.5">
                    <Label className="!mb-0">Recent imports</Label>
                    <div className="text-[0.625rem] text-ink-3 font-mono">LAST 90 DAYS</div>
                </div>
                <div className="flex flex-col gap-1.5">
                    {recents.length === 0 && (
                        <div className="text-xs text-ink-3 py-4 text-center">No recent imports</div>
                    )}
                    {recents.map((r) => (
                        <div
                            key={r.id}
                            className="px-3 py-2.5 rounded-lg bg-[rgba(255,255,255,0.02)] border border-line flex items-center gap-3"
                        >
                            <div
                                className="w-8 h-8 rounded-[7px] flex items-center justify-center flex-shrink-0 text-accent-cyan"
                                style={{background: "color-mix(in oklab, var(--cyan) 14%, transparent)"}}
                            >
                                <I.folder size={14}/>
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="text-xs font-semibold overflow-hidden text-ellipsis whitespace-nowrap">
                                    {r.name}
                                </div>
                                <div className="text-[0.625rem] text-ink-3 mt-0.5">
                                    {r.source} · {formatSize(r.sizeBytes)} · {timeAgo(r.importedAt)}
                                </div>
                            </div>
                            <button
                                type="button"
                                className="bg-transparent border border-line text-ink-2 px-2.5 py-1 rounded-md text-[0.6875rem] cursor-pointer hover:bg-[rgba(255,255,255,0.04)]"
                                onClick={() => handleReimport(r.id)}
                                disabled={installing}
                            >
                                Reimport
                            </button>
                        </div>
                    ))}
                </div>
            </div>
        </ModalShell>
    );
}
