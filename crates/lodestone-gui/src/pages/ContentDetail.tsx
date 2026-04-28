import {useCallback, useEffect, useState} from "react";
import {useNavigate, useParams} from "react-router-dom";
import {Button, Spinner} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import {I} from "../components/shell/icons";
import DetailHero from "../components/discover/DetailHero";
import DetailSidebar from "../components/discover/DetailSidebar";
import SummaryTab from "../components/discover/SummaryTab";
import GalleryTab from "../components/discover/GalleryTab";
import VersionsTab from "../components/discover/VersionsTab";
import DependenciesTab from "../components/discover/DependenciesTab";
import type {ContentItem, Dependency, ProjectVersion} from "../types/content";

type DetailTab = "summary" | "gallery" | "versions" | "dependencies";

const tabs: {key: DetailTab; label: string; icon: keyof typeof I}[] = [
    {key: "summary", label: "Summary", icon: "compass"},
    {key: "gallery", label: "Gallery", icon: "image"},
    {key: "versions", label: "Versions", icon: "pkg"},
    {key: "dependencies", label: "Dependencies", icon: "box"},
];

/** Detect whether a content item is a modpack (has `has_server_pack` field). */
function isModpack(item: ContentItem): boolean {
    return "has_server_pack" in item;
}

/** CurseForge needs numeric ID for version lookups; Modrinth accepts slug or ID. */
function versionProjectId(item: ContentItem, platform: string | undefined): string {
    if (platform?.toLowerCase() === "curseforge") return item.id;
    return item.slug || item.id;
}

export default function ContentDetail() {
    const {platform, id} = useParams<{platform: string; id: string}>();
    const navigate = useNavigate();
    const [item, setItem] = useState<ContentItem | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [tab, setTab] = useState<DetailTab>("summary");
    const [installing, setInstalling] = useState(false);
    const [installError, setInstallError] = useState<string | null>(null);

    useEffect(() => {
        if (!platform || !id) return;
        setLoading(true);
        setError(null);

        invoke<ContentItem | null>("get_content", {
            id,
            platform,
            contentType: "mod", // first try mod; if null, try modpack
        })
            .then(async result => {
                if (result) {
                    setItem(result);
                    return;
                }
                // Fallback: try modpack
                const packResult = await invoke<ContentItem | null>("get_content", {
                    id,
                    platform,
                    contentType: "modpack",
                });
                setItem(packResult);
            })
            .catch(e => setError(String(e)))
            .finally(() => setLoading(false));
    }, [platform, id]);

    // Listen for completion events during install
    useEffect(() => {
        if (!installing) return;

        const unlisten = listen<{instanceId: number; instanceName: string}>("install-completed", () => {
            setInstalling(false);
            setInstallError(null);
        });

        return () => {
            unlisten.then((fn) => fn());
        };
    }, [installing]);

    const handleInstallModpack = useCallback(async () => {
        if (!item || !platform) return;
        setInstalling(true);
        setInstallError(null);

        try {
            const pid = versionProjectId(item, platform);
            // Fetch versions to get the latest version ID
            const versions = await invoke<ProjectVersion[]>("get_project_versions", {
                projectId: pid,
                platform: platform.toLowerCase(),
            });
            if (!versions || versions.length === 0) {
                throw new Error("No versions available for this modpack");
            }
            const latestVersion = versions[0];

            await invoke("install_modpack_from_discover", {
                projectId: pid,
                versionId: latestVersion.id,
                platform: platform.toLowerCase(),
            });
        } catch (e) {
            setInstallError(String(e));
            setInstalling(false);
        }
    }, [item, platform]);

    if (loading) {
        return (
            <div className="flex-1 flex items-center justify-center">
                <Spinner size="lg" color="success"/>
            </div>
        );
    }

    if (error || !item) {
        return (
            <div className="flex-1 flex flex-col items-center justify-center gap-4">
                <I.x size={40} style={{color: "var(--ink-4)"}}/>
                <span style={{fontSize: 14, color: "var(--ink-2)"}}>
                    {error || "Content not found"}
                </span>
                <Button
                    variant="bordered"
                    size="sm"
                    className="border-line text-ink-2"
                    onPress={() => navigate(-1)}
                    startContent={<I.chevRight size={14} style={{transform: "rotate(180deg)"}}/>}
                >
                    Back to Discover
                </Button>
            </div>
        );
    }

    const loaders = "loaders" in item ? (item as {loaders: string[]}).loaders : [];
    const deps: Dependency[] = "dependencies" in item ? (item as {dependencies: Dependency[]}).dependencies : [];
    const isPack = isModpack(item);

    return (
        <div className="flex-1 flex flex-col overflow-y-auto" style={{background: "var(--bg-0)"}}>
            {/* Back button */}
            <div style={{padding: "10px 28px 0"}}>
                <Button
                    variant="light"
                    size="sm"
                    className="text-ink-3"
                    onPress={() => navigate(-1)}
                    startContent={<I.chevRight size={14} style={{transform: "rotate(180deg)"}}/>}
                >
                    Back to Discover
                </Button>
            </div>

            {/* Hero */}
            <DetailHero
                item={item}
                onInstall={isPack ? handleInstallModpack : undefined}
                installing={installing}
            />

            {/* Install error banner */}
            {installError && (
                <div className="mx-7 mt-2 p-3 rounded-lg border border-red-500/30 bg-red-500/5 text-xs text-red-400">
                    {installError}
                </div>
            )}

            {/* Tabs */}
            <div className="flex border-b border-line" style={{padding: "0 28px"}}>
                {tabs.map(t => {
                    const isActive = t.key === tab;
                    const TabIcon = I[t.icon];
                    return (
                        <button
                            key={t.key}
                            onClick={() => setTab(t.key)}
                            className="flex items-center gap-2 cursor-pointer bg-transparent"
                            style={{
                                padding: "10px 16px",
                                fontSize: 13,
                                fontWeight: isActive ? 600 : 500,
                                color: isActive ? "var(--mc-green)" : "var(--ink-2)",
                                border: "none",
                                borderBottom: isActive
                                    ? "2px solid var(--mc-green)"
                                    : "2px solid transparent",
                                transition: "all 0.12s",
                            }}
                        >
                            <TabIcon size={14}/>
                            {t.label}
                        </button>
                    );
                })}
            </div>

            {/* Content area: 2-column layout */}
            <div
                style={{
                    display: "grid",
                    gridTemplateColumns: "2fr 1fr",
                    gap: 18,
                    padding: "20px 28px 40px",
                }}
            >
                {/* Main column */}
                <div>
                    {tab === "summary" && (
                        <SummaryTab description={item.description} summary={item.summary}/>
                    )}
                    {tab === "gallery" && <GalleryTab images={item.gallery} title={item.title}/>}
                    {tab === "versions" && (
                        <VersionsTab
                            projectId={versionProjectId(item, platform)}
                            platform={platform?.toLowerCase() ?? "modrinth"}
                            isModpack={isPack}
                        />
                    )}
                    {tab === "dependencies" && (
                        <DependenciesTab
                            deps={deps}
                            projectId={versionProjectId(item, platform)}
                            platform={platform?.toLowerCase()}
                        />
                    )}
                </div>

                {/* Sidebar */}
                <DetailSidebar item={item} loaders={loaders}/>
            </div>
        </div>
    );
}
