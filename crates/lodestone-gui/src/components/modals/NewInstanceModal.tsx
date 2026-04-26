import {useCallback, useEffect, useState} from "react";
import {invoke} from "@tauri-apps/api/core";
import ModalShell from "./ModalShell";
import {FooterBtn, InputRow, Label, VersionDropdown} from "./primitives";
import {I} from "../shell/icons";
import type {McVersion} from "../../hooks/useMinecraftVersions";

type Props = {isOpen: boolean; onClose: () => void; onCreated?: () => void};

const loaders = [
    {id: "vanilla", name: "Vanilla", color: "#9aa4ae", desc: "Unmodded Minecraft", icon: "/icons/vanilla.svg"},
    {id: "fabric", name: "Fabric", color: "#c9b88c", desc: "Lightweight, fast startup", icon: "/icons/fabric.svg"},
    {id: "forge", name: "Forge", color: "#5a7fb3", desc: "Largest mod ecosystem", icon: "/icons/forge.svg"},
    {id: "neoforge", name: "NeoForge", color: "#e08548", desc: "Modern Forge fork", icon: "/icons/neoforge.svg"},
    {id: "quilt", name: "Quilt", color: "#c16fa3", desc: "Fabric-compatible", icon: "/icons/quilt.svg"},
];

interface LoaderVersionsResponse {
    versions: string[];
    recommended: string | null;
}

interface JavaInfo {
    majorVersion: number;
    label: string;
}

export default function NewInstanceModal({isOpen, onClose, onCreated}: Props) {
    const [name, setName] = useState("");
    const [mcVersion, setMcVersion] = useState("");
    const [loader, setLoader] = useState("vanilla");
    const [loaderVersion, setLoaderVersion] = useState<string | null>(null);
    const [creating, setCreating] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Fetched data
    const [mcVersions, setMcVersions] = useState<string[]>([]);
    const [mcVersionsLoading, setMcVersionsLoading] = useState(false);
    const [loaderVersions, setLoaderVersions] = useState<string[]>([]);
    const [loaderVersionsLoading, setLoaderVersionsLoading] = useState(false);
    const [javaInfo, setJavaInfo] = useState<JavaInfo | null>(null);
    const [showSnapshots, setShowSnapshots] = useState(false);
    const [allMcVersions, setAllMcVersions] = useState<McVersion[]>([]);

    // Fetch MC versions on open
    useEffect(() => {
        if (!isOpen) return;
        setMcVersionsLoading(true);
        invoke<McVersion[]>("get_minecraft_versions")
            .then((versions) => {
                setAllMcVersions(versions);
                const releases = versions.filter((v) => v.version_type === "Release");
                setMcVersions(releases.map((v) => v.id));
                if (releases.length > 0 && !mcVersion) {
                    setMcVersion(releases[0].id);
                }
            })
            .catch((e) => setError(String(e)))
            .finally(() => setMcVersionsLoading(false));
    }, [isOpen]);

    // Update displayed MC versions when snapshot toggle changes
    useEffect(() => {
        if (showSnapshots) {
            setMcVersions(allMcVersions.map((v) => v.id));
        } else {
            setMcVersions(allMcVersions.filter((v) => v.version_type === "Release").map((v) => v.id));
        }
    }, [showSnapshots, allMcVersions]);

    // Fetch Java info when MC version changes
    useEffect(() => {
        if (!mcVersion) return;
        invoke<JavaInfo>("get_java_for_version", {minecraftVersion: mcVersion})
            .then(setJavaInfo)
            .catch(() => setJavaInfo(null));
    }, [mcVersion]);

    // Fetch loader versions when MC version or loader changes
    useEffect(() => {
        if (!mcVersion || loader === "vanilla") {
            setLoaderVersions([]);
            setLoaderVersion(null);
            return;
        }
        setLoaderVersionsLoading(true);
        invoke<LoaderVersionsResponse>("get_loader_versions", {
            loader,
            minecraftVersion: mcVersion,
        })
            .then((resp) => {
                setLoaderVersions(resp.versions);
                setLoaderVersion(resp.recommended ?? resp.versions[0] ?? null);
            })
            .catch((e) => {
                setError(String(e));
                setLoaderVersions([]);
                setLoaderVersion(null);
            })
            .finally(() => setLoaderVersionsLoading(false));
    }, [mcVersion, loader]);

    // Reset form when modal opens
    useEffect(() => {
        if (isOpen) {
            setName("");
            setMcVersion("");
            setLoader("vanilla");
            setLoaderVersion(null);
            setError(null);
            setJavaInfo(null);
        }
    }, [isOpen]);

    const latestMcVersion = mcVersions[0] ?? "";

    const handleCreate = useCallback(async () => {
        if (!name.trim()) {
            setError("Please enter an instance name.");
            return;
        }
        if (!mcVersion) {
            setError("Please select a Minecraft version.");
            return;
        }
        setCreating(true);
        setError(null);
        try {
            await invoke("create_instance", {
                request: {
                    name: name.trim(),
                    minecraftVersion: mcVersion,
                    loader,
                    loaderVersion: loader === "vanilla" ? null : loaderVersion,
                    javaVersion: javaInfo ? String(javaInfo.majorVersion) : null,
                },
            });
            onCreated?.();
            onClose();
        } catch (e) {
            setError(typeof e === "string" ? e : String(e));
        } finally {
            setCreating(false);
        }
    }, [name, mcVersion, loader, loaderVersion, javaInfo, onClose, onCreated]);

    return (
        <ModalShell
            isOpen={isOpen}
            onClose={onClose}
            title="New Instance"
            subtitle="Spin up a fresh Minecraft profile. Pick version and loader, we'll do the rest."
            icon={I.plus}
            size="3xl"
            footer={
                <>
                    <FooterBtn onClick={onClose}>Cancel</FooterBtn>
                    <FooterBtn
                        primary
                        onClick={handleCreate}
                        loading={creating}
                        disabled={creating || !name.trim() || !mcVersion}
                    >
                        Create Instance
                    </FooterBtn>
                </>
            }
        >
            {/* Error banner */}
            {error && (
                <div className="mb-4 px-3.5 py-2.5 rounded-lg border border-[rgba(255,107,107,0.3)] bg-[rgba(255,107,107,0.08)] text-[0.8125rem] text-[#ff8a8a] flex items-center gap-2">
                    <I.shield size={14} className="flex-shrink-0"/>
                    <span className="flex-1">{error}</span>
                    <button
                        className="text-ink-3 hover:text-ink-1 bg-transparent border-none cursor-pointer p-0"
                        onClick={() => setError(null)}
                    >
                        <I.close size={12}/>
                    </button>
                </div>
            )}

            <Label>Instance name</Label>
            <InputRow value={name} placeholder="My New Instance" onChange={setName}/>

            {/* Version + Java row */}
            <div className="grid grid-cols-2 gap-3.5 mt-4">
                <div>
                    <div className="flex items-center justify-between mb-1.5">
                        <Label className="!mb-0">Minecraft version</Label>
                        <label className="flex items-center gap-1.5 cursor-pointer">
                            <input
                                type="checkbox"
                                checked={showSnapshots}
                                onChange={(e) => setShowSnapshots(e.target.checked)}
                                className="accent-[var(--mc-green)] w-3 h-3"
                            />
                            <span className="text-[0.5625rem] text-ink-3 font-mono">SNAPSHOTS</span>
                        </label>
                    </div>
                    <VersionDropdown
                        versions={mcVersions}
                        selected={mcVersion}
                        onChange={setMcVersion}
                        latestVersion={latestMcVersion}
                        placeholder="Select version"
                        loading={mcVersionsLoading}
                    />
                </div>
                <div>
                    <Label>Java</Label>
                    <div className="px-3 py-2.5 rounded-[9px] border border-line bg-[rgba(0,0,0,0.3)] flex items-center justify-between">
                        <div>
                            <div className="text-[0.8125rem] text-ink-0">
                                {javaInfo ? javaInfo.label : "—"}
                            </div>
                            <div className="text-[0.625rem] text-ink-3 mt-px">Auto-matched to version</div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Loader cards */}
            <Label className="mt-[18px]">Mod loader</Label>
            <div className="grid grid-cols-5 gap-2">
                {loaders.map((l) => {
                    const selected = l.id === loader;
                    return (
                        <div
                            key={l.id}
                            className="relative p-3 rounded-[10px] cursor-pointer border transition-all"
                            style={{
                                borderColor: selected ? l.color : "var(--line)",
                                borderWidth: selected ? 1.5 : 1,
                                background: selected
                                    ? `color-mix(in oklab, ${l.color} 10%, transparent)`
                                    : "rgba(255,255,255,0.02)",
                            }}
                            onClick={() => setLoader(l.id)}
                        >
                            {l.id === "vanilla" ? (
                                <img
                                    src={l.icon}
                                    alt={l.name}
                                    className="w-7 h-7 rounded-[7px] mb-2"
                                    style={{
                                        boxShadow: selected ? `0 0 12px ${l.color}80` : "none",
                                    }}
                                />
                            ) : (
                                <div
                                    className="w-7 h-7 mb-2"
                                    style={{
                                        boxShadow: selected ? `0 0 12px ${l.color}80` : "none",
                                        WebkitMaskImage: `url(${l.icon})`,
                                        WebkitMaskSize: "contain",
                                        WebkitMaskRepeat: "no-repeat",
                                        WebkitMaskPosition: "center",
                                        maskImage: `url(${l.icon})`,
                                        maskSize: "contain",
                                        maskRepeat: "no-repeat",
                                        maskPosition: "center",
                                        backgroundColor: l.color,
                                    }}
                                />
                            )}
                            <div className="text-xs font-semibold">{l.name}</div>
                            <div className="text-[0.625rem] text-ink-3 mt-0.5 leading-tight">
                                {l.desc}
                            </div>
                            {selected && (
                                <div
                                    className="absolute top-1.5 right-1.5"
                                    style={{color: l.color}}
                                >
                                    <I.check size={12}/>
                                </div>
                            )}
                        </div>
                    );
                })}
            </div>

            {/* Loader version dropdown — only for non-vanilla */}
            {loader !== "vanilla" && (
                <div className="mt-3">
                    <Label>
                        {loaders.find((l) => l.id === loader)?.name ?? "Loader"} version
                    </Label>
                    <VersionDropdown
                        versions={loaderVersions}
                        selected={loaderVersion ?? ""}
                        onChange={setLoaderVersion}
                        latestVersion={loaderVersions[0]}
                        placeholder="Select loader version"
                        loading={loaderVersionsLoading}
                    />
                </div>
            )}

            {/* Quick-start templates placeholder (not implemented per plan) */}
            <div
                className="mt-[18px] p-3.5 rounded-xl opacity-50 pointer-events-none"
                style={{
                    background: "rgba(34,255,132,0.04)",
                    border: "1px dashed color-mix(in oklab, var(--mc-green) 20%, transparent)",
                }}
            >
                <div className="flex items-center justify-between mb-1">
                    <div>
                        <div className="text-xs font-semibold text-mc-green">
                            Quick start from template
                        </div>
                        <div className="text-[0.6875rem] text-ink-3 mt-0.5">
                            Coming soon — curated setups for common playstyles
                        </div>
                    </div>
                    <div className="text-[0.625rem] text-ink-3 font-mono">COMING SOON</div>
                </div>
            </div>
        </ModalShell>
    );
}
