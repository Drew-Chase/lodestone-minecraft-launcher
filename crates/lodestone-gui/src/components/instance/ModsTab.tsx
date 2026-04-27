import {useCallback, useEffect, useState} from "react";
import {Button, Dropdown, DropdownTrigger, DropdownMenu, DropdownItem, Input, Popover, PopoverTrigger, PopoverContent, Spinner} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {open as shellOpen} from "@tauri-apps/plugin-shell";
import {Switch} from "../Switch";
import {I} from "../shell/icons";
import {cardSurfaceStyle, cardHoverClass, type Instance} from "../library/instances";
import SourceTabs from "../discover/SourceTabs";
import SortSelect from "../discover/SortSelect";
import ViewModeToggle from "../discover/ViewModeToggle";
import FilterDrawer from "../discover/FilterDrawer";
import SourceBadge from "../discover/SourceBadge";
import DetailSidebar from "../discover/DetailSidebar";
import SummaryTab from "../discover/SummaryTab";
import GalleryTab from "../discover/GalleryTab";
import DependenciesTab from "../discover/DependenciesTab";
import useContentSearch from "../../hooks/useContentSearch";
import {usePersistedState} from "../../hooks/usePersistedState";
import {defaultFilterState, formatCount, formatSize, timeAgo} from "../../types/content";
import type {ContentItem, Dependency, FilterState, ProjectVersion, SortKey, SourceKey, ViewMode} from "../../types/content";

type Props = {
    instance: Instance;
};

interface ModEntry {
    fileName: string;
    fileSizeBytes: number;
    enabled: boolean;
    lastModified: string;
}

interface InstalledModRecord {
    id: number;
    instanceId: number;
    fileName: string;
    modrinthId: string | null;
    curseforgeId: string | null;
    versionId: string | null;
    projectName: string | null;
    iconUrl: string | null;
    installedAt: string;
}

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function displayName(fileName: string): string {
    return fileName.replace(/\.jar\.disabled$/, "").replace(/\.jar$/, "");
}

export default function ModsTab({instance}: Props) {
    const [mods, setMods] = useState<ModEntry[]>([]);
    const [modRecords, setModRecords] = useState<InstalledModRecord[]>([]);
    const [filter, setFilter] = useState("");
    const [browsing, setBrowsing] = useState(false);
    const [detailItem, setDetailItem] = useState<ContentItem | null>(null);

    const fetchMods = useCallback(async () => {
        try {
            const result = await invoke<ModEntry[]>("list_instance_mods", {instancePath: instance.instancePath});
            setMods(result);
        } catch { setMods([]); }
        try {
            const records = await invoke<InstalledModRecord[]>("get_installed_mods_info", {instanceId: instance.id});
            setModRecords(records);
        } catch { setModRecords([]); }
    }, [instance.instancePath, instance.id]);

    useEffect(() => { fetchMods(); }, [fetchMods]);

    // Find DB record for a given file name
    const findRecord = (fileName: string): InstalledModRecord | undefined =>
        modRecords.find(r => r.fileName === fileName || r.fileName === fileName.replace(/\.disabled$/, ""));

    // Open mod detail for a tracked mod
    const openModDetail = async (rec: InstalledModRecord) => {
        const platform = rec.modrinthId ? "modrinth" : rec.curseforgeId ? "curseforge" : null;
        const projectId = rec.modrinthId || rec.curseforgeId;
        if (!platform || !projectId) return;
        try {
            const item = await invoke<ContentItem | null>("get_content", {
                id: projectId, platform, contentType: "mod",
            });
            if (item) setDetailItem(item);
        } catch { /* ignore */ }
    };

    const handleToggle = async (mod: ModEntry, enabled: boolean) => {
        await invoke("toggle_mod", {instancePath: instance.instancePath, fileName: mod.fileName, enabled});
        fetchMods();
    };

    const handleDelete = async (mod: ModEntry) => {
        await invoke("delete_mod", {instanceId: instance.id, instancePath: instance.instancePath, fileName: mod.fileName});
        fetchMods();
    };

    const handleOpenFolder = () => {
        invoke("open_directory", {path: `${instance.instancePath}${instance.instancePath.includes("/") ? "/" : "\\"}mods`});
    };

    const filtered = mods.filter((m) => displayName(m.fileName).toLowerCase().includes(filter.toLowerCase()));

    if (browsing) {
        return <ModBrowser instance={instance} onBack={() => { setBrowsing(false); fetchMods(); }}/>;
    }

    // Detail overlay for clicking an installed mod
    if (detailItem) {
        return (
            <div className="flex-1 flex flex-col overflow-hidden relative">
                <div className="fixed top-0 right-0 bottom-0 z-50 flex flex-col" style={{left: 68, background: "var(--bg-0)"}}>
                    <ModDetailOverlay
                        item={detailItem}
                        instance={instance}
                        onBack={() => { setDetailItem(null); fetchMods(); }}
                        onInstalled={() => {}}
                    />
                </div>
            </div>
        );
    }

    return (
        <div className="flex-1 flex flex-col min-h-0 px-7 pt-5 pb-5">
            <div className="flex-shrink-0 flex gap-2.5 mb-3.5">
                <Input placeholder="Filter mods…" size="sm" value={filter} onValueChange={setFilter}
                    classNames={{base: "flex-1", inputWrapper: "bg-[rgba(255,255,255,0.04)] border border-line"}}
                    startContent={<I.search size={14}/>}/>
                <Button variant="bordered" size="sm" startContent={<I.folder size={13}/>} onPress={handleOpenFolder}>Open Folder</Button>
                <Button variant="bordered" size="sm" startContent={<I.refresh size={13}/>} onPress={fetchMods}>Refresh</Button>
                <Button color="success" size="sm" className="font-bold" startContent={<I.plus size={12}/>} onPress={() => setBrowsing(true)}>Add Mod</Button>
            </div>

            {mods.length === 0 ? (
                <div className="flex-1 flex items-center justify-center text-ink-3 text-sm">
                    <div className="text-center">
                        <I.box size={32} className="mx-auto mb-3 opacity-40"/>
                        <div>No mods installed</div>
                        <div className="text-xs mt-1 text-ink-4 mb-3">Add .jar files to the mods folder to get started</div>
                        <Button color="success" size="sm" className="font-bold" startContent={<I.search size={12}/>} onPress={() => setBrowsing(true)}>Browse Mods</Button>
                    </div>
                </div>
            ) : (
                <div className="flex-1 min-h-0 overflow-y-auto rounded-md border border-line" style={cardSurfaceStyle}>
                    {filtered.map((m, i) => {
                        const rec = findRecord(m.fileName);
                        const isClickable = !!rec;
                        return (
                            <div
                                key={m.fileName}
                                className={`flex items-center gap-3.5 px-4 py-3 transition-colors hover:bg-[rgba(255,255,255,0.02)] ${i < filtered.length - 1 ? "border-b border-line" : ""} ${isClickable ? "cursor-pointer" : ""}`}
                                onClick={isClickable ? () => openModDetail(rec) : undefined}
                            >
                                {rec?.iconUrl ? (
                                    <img src={rec.iconUrl} alt="" className="flex-shrink-0 w-8 h-8 rounded-lg object-cover"/>
                                ) : (
                                    <div className="flex-shrink-0 w-8 h-8 rounded-lg flex items-center justify-center border border-line bg-[rgba(255,255,255,0.04)] text-ink-2"><I.box size={14}/></div>
                                )}
                                <div className="min-w-0 flex-1">
                                    <div className="text-[0.8125rem] font-semibold truncate">{rec?.projectName || displayName(m.fileName)}</div>
                                    <div className="text-[0.6875rem] text-ink-3 mt-0.5 font-mono flex items-center gap-2">
                                        <span>{formatBytes(m.fileSizeBytes)}</span>
                                        {rec && <span className="text-ink-4">· {rec.modrinthId ? "Modrinth" : "CurseForge"}</span>}
                                    </div>
                                </div>
                                <div onClick={e => e.stopPropagation()} className="flex items-center gap-1.5">
                                    <Switch isSelected={m.enabled} onValueChange={(v) => handleToggle(m, v)} size="sm" color="success" aria-label={`Toggle ${displayName(m.fileName)}`}/>
                                    <Button isIconOnly variant="bordered" size="sm" aria-label="Delete" className="w-7 h-7 min-w-0" onPress={() => handleDelete(m)}><I.trash size={13}/></Button>
                                </div>
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

// ---------------------------------------------------------------------------
// Full mod browser — reuses Discover components + detail overlay
// ---------------------------------------------------------------------------

function ModBrowser({instance, onBack}: { instance: Instance; onBack: () => void }) {
    const [source, setSource] = useState<SourceKey>("modrinth");
    const [query, setQuery] = useState("");
    const [sort, setSort] = useState<SortKey>("downloads");
    const [viewMode, setViewMode] = usePersistedState<ViewMode>("modBrowser.viewMode", "grid");
    const [showFilter, setShowFilter] = useState(false);
    const [filters, setFilters] = useState<FilterState>({
        ...defaultFilterState,
        loaders: [instance.loader.toLowerCase()],
        versions: [instance.mc],
    });
    const [installing, setInstalling] = useState<Set<string>>(new Set());
    const [installed, setInstalled] = useState<Set<string>>(new Set());
    const [detailItem, setDetailItem] = useState<ContentItem | null>(null);

    const {results, loading, error, hasMore, sentinelRef} = useContentSearch({
        query, sort, source, contentType: "mod", filters,
    });

    const activeFilterCount = filters.categories.length + filters.loaders.length + filters.versions.length + filters.environment.length;

    const handleInstall = async (item: ContentItem) => {
        setInstalling((prev) => new Set([...prev, item.id]));
        try {
            await invoke("install_mod", {
                instanceId: instance.id, instancePath: instance.instancePath, projectId: item.id,
                platform: item.platform.toLowerCase(), mcVersion: instance.mc, loader: instance.loader.toLowerCase(),
                projectName: item.title,
            });
            setInstalled((prev) => new Set([...prev, item.id]));
        } catch (e) { console.error("Install failed:", e); }
        finally {
            setInstalling((prev) => { const n = new Set(prev); n.delete(item.id); return n; });
        }
    };

    const openDetail = async (item: ContentItem) => {
        try {
            const full = await invoke<ContentItem | null>("get_content", {
                id: item.slug || item.id,
                platform: item.platform.toLowerCase(),
                contentType: "mod",
            });
            setDetailItem(full ?? item);
        } catch {
            setDetailItem(item);
        }
    };

    return (
        <div className="flex-1 flex flex-col overflow-hidden relative">
            {/* Toolbar */}
            <div className="flex-shrink-0 flex items-center gap-3 flex-wrap border-b border-line" style={{padding: "14px 28px"}}>
                <Button variant="bordered" size="sm" startContent={<I.chevRight size={13} className="rotate-180"/>} onPress={onBack}>Back</Button>
                <div className="relative" style={{width: 300}}>
                    <Input placeholder="Search mods…" value={query} onValueChange={setQuery} startContent={<I.search size={14} style={{color: "var(--ink-3)"}}/>}
                        size="sm" variant="bordered" classNames={{inputWrapper: "border-line bg-[rgba(0,0,0,0.3)]"}}/>
                </div>
                <SourceTabs active={source} contentType="mod" onChange={setSource}/>
                <div className="flex-1"/>
                <SortSelect value={sort} onChange={setSort}/>
                <button onClick={() => setShowFilter(true)} className="cursor-pointer flex items-center gap-2"
                    style={{padding: "7px 12px", fontSize: 12, fontWeight: 600, borderRadius: 8,
                        border: activeFilterCount > 0 ? "1px solid rgba(34,255,132,0.4)" : "1px solid var(--line)",
                        background: activeFilterCount > 0 ? "rgba(34,255,132,0.08)" : "transparent",
                        color: activeFilterCount > 0 ? "var(--mc-green)" : "var(--ink-2)"}}>
                    <I.filter size={14}/> Filters
                    {activeFilterCount > 0 && <span style={{padding: "1px 6px", borderRadius: 999, fontSize: 10, fontWeight: 700, background: "rgba(34,255,132,0.14)", color: "var(--mc-green)"}}>{activeFilterCount}</span>}
                </button>
                <ViewModeToggle value={viewMode} onChange={setViewMode}/>
                <span style={{fontSize: 12, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>{results.length} result{results.length !== 1 ? "s" : ""}</span>
            </div>

            {/* Results */}
            <div className="flex-1 overflow-y-auto" style={{padding: "20px 28px 40px"}}>
                {error && <div className="border border-line rounded-xl flex items-center gap-3 mb-3.5" style={{padding: 16, background: "rgba(255,80,80,0.06)", borderColor: "rgba(255,80,80,0.2)", color: "#ff5050", fontSize: 12}}><I.x size={14}/> {error}</div>}
                {loading && results.length === 0 ? (
                    <div className="flex items-center justify-center py-20"><Spinner size="lg" color="success"/></div>
                ) : results.length === 0 && !loading ? (
                    <div className="flex flex-col items-center justify-center py-20 gap-3">
                        <I.search size={40} style={{color: "var(--ink-4)"}}/>
                        <span style={{fontSize: 14, color: "var(--ink-3)"}}>No results found</span>
                    </div>
                ) : (
                    <>
                        {viewMode === "grid" && (
                            <div style={{display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: 14}}>
                                {results.map((item, i) => <InstallableCard key={`${item.id}-${i}`} item={item} installing={installing.has(item.id)} installed={installed.has(item.id)} onInstall={() => handleInstall(item)} onClick={() => openDetail(item)}/>)}
                            </div>
                        )}
                        {viewMode === "compact" && (
                            <div style={{display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 10}}>
                                {results.map((item, i) => <InstallableCompact key={`${item.id}-${i}`} item={item} installing={installing.has(item.id)} installed={installed.has(item.id)} onInstall={() => handleInstall(item)} onClick={() => openDetail(item)}/>)}
                            </div>
                        )}
                        {viewMode === "table" && (
                            <div className="rounded-md border border-line overflow-hidden" style={cardSurfaceStyle}>
                                {results.map((item, i) => <InstallableRow key={`${item.id}-${i}`} item={item} isLast={i === results.length - 1} installing={installing.has(item.id)} installed={installed.has(item.id)} onInstall={() => handleInstall(item)} onClick={() => openDetail(item)}/>)}
                            </div>
                        )}
                        {hasMore && <div ref={sentinelRef} className="flex items-center justify-center py-6">{loading && <Spinner size="sm" color="success"/>}</div>}
                        {!hasMore && results.length > 0 && <div className="flex items-center justify-center py-6" style={{fontSize: 12, color: "var(--ink-4)", fontFamily: "var(--mono)"}}>End of results</div>}
                    </>
                )}
            </div>
            <FilterDrawer isOpen={showFilter} onClose={() => setShowFilter(false)} filters={filters} onChange={setFilters} contentType="mod" source={source}/>

            {/* Detail overlay — covers everything, browser stays mounted underneath */}
            {detailItem && (
                <div className="fixed top-0 right-0 bottom-0 z-50 flex flex-col" style={{left: 68, background: "var(--bg-0)"}}>
                    <ModDetailOverlay
                        item={detailItem}
                        instance={instance}
                        onBack={() => setDetailItem(null)}
                        onInstalled={(id) => setInstalled((prev) => new Set([...prev, id]))}
                    />
                </div>
            )}
        </div>
    );
}

// ---------------------------------------------------------------------------
// Mod detail overlay — like ContentDetail but with Install buttons on versions
// ---------------------------------------------------------------------------

type DetailTabKey = "summary" | "gallery" | "versions" | "dependencies";

function platformUrl(item: ContentItem): string | null {
    const p = item.platform.toLowerCase();
    const slug = item.slug || item.id;
    if (p === "modrinth") return `https://modrinth.com/mod/${slug}`;
    if (p === "curseforge") return `https://www.curseforge.com/minecraft/mc-mods/${slug}`;
    return null;
}

function ModDetailOverlay({item: initialItem, instance, onBack, onInstalled}: {
    item: ContentItem;
    instance: Instance;
    onBack: () => void;
    onInstalled: (id: string) => void;
}) {
    const [item, setItem] = useState(initialItem);
    const [history, setHistory] = useState<ContentItem[]>([]);
    const [tab, setTab] = useState<DetailTabKey>("summary");
    const [installState, setInstallState] = useState<"idle" | "installing" | "installed">("idle");
    const [installedRecord, setInstalledRecord] = useState<InstalledModRecord | null>(null);

    const navigateToMod = (newItem: ContentItem) => {
        setHistory(prev => [...prev, item]);
        setItem(newItem);
        setTab("summary");
        setInstallState("idle");
        setInstalledRecord(null);
    };

    const goBack = () => {
        if (history.length > 0) {
            const prev = history[history.length - 1];
            setHistory(h => h.slice(0, -1));
            setItem(prev);
            setTab("summary");
            setInstallState("idle");
            setInstalledRecord(null);
        } else {
            onBack();
        }
    };
    const loaders = "loaders" in item ? (item as { loaders: string[] }).loaders : [];
    const deps: Dependency[] = "dependencies" in item ? (item as { dependencies: Dependency[] }).dependencies : [];
    const url = platformUrl(item);

    // Look up if this mod is already installed
    useEffect(() => {
        invoke<InstalledModRecord[]>("get_installed_mods_info", {instanceId: instance.id})
            .then(records => {
                const match = records.find(r =>
                    (r.modrinthId && r.modrinthId === item.id) ||
                    (r.curseforgeId && r.curseforgeId === item.id) ||
                    (r.modrinthId && r.modrinthId === item.slug) ||
                    (r.curseforgeId && r.curseforgeId === item.slug)
                );
                if (match) {
                    setInstalledRecord(match);
                    setInstallState("installed");
                }
            })
            .catch(() => {});
    }, [instance.id, item.id, item.slug]);

    const handleInstall = async () => {
        setInstallState("installing");
        try {
            await invoke("install_mod", {
                instanceId: instance.id, instancePath: instance.instancePath, projectId: item.id,
                platform: item.platform.toLowerCase(), mcVersion: instance.mc, loader: instance.loader.toLowerCase(),
                projectName: item.title,
            });
            setInstallState("installed");
            onInstalled(item.id);
        } catch (e) {
            console.error("Install failed:", e);
            setInstallState("idle");
        }
    };

    const detailTabs: { key: DetailTabKey; label: string; icon: keyof typeof I }[] = [
        {key: "summary", label: "Summary", icon: "compass"},
        {key: "gallery", label: "Gallery", icon: "image"},
        {key: "versions", label: "Versions", icon: "pkg"},
        {key: "dependencies", label: "Dependencies", icon: "box"},
    ];

    // Intercept browser back button (mouse back) to go back in history or close
    useEffect(() => {
        window.history.pushState({modDetail: true}, "");
        const handlePopState = () => { goBack(); };
        window.addEventListener("popstate", handlePopState);
        return () => { window.removeEventListener("popstate", handlePopState); };
    }, [item.id]); // eslint-disable-line react-hooks/exhaustive-deps

    return (
        <div className="flex-1 flex flex-col overflow-y-auto" style={{background: "var(--bg-0)"}}>
            {/* Back + actions bar — pt-[36px] clears the title bar drag region */}
            <div className="flex items-center gap-2" style={{padding: "36px 28px 0"}}>
                <Button variant="light" size="sm" className="text-ink-3" onPress={goBack}
                    startContent={<I.chevRight size={14} style={{transform: "rotate(180deg)"}}/>}>
                    {history.length > 0 ? "Back" : "Back to Browse"}
                </Button>
                <div className="flex-1"/>
                {url && (
                    <Button variant="bordered" size="sm" className="border-line text-ink-2"
                        onPress={() => shellOpen(url)}
                        startContent={<I.external size={13}/>}>
                        Open on {item.platform}
                    </Button>
                )}
                {installState === "installed" ? (
                    <Button size="sm" variant="flat" className="font-bold text-mc-green" isDisabled startContent={<I.check size={12}/>}>Installed</Button>
                ) : installState === "installing" ? (
                    <Button size="sm" variant="bordered" isDisabled startContent={<div className="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"/>}>Installing</Button>
                ) : (
                    <Button color="success" size="sm" className="font-bold" startContent={<I.download size={12}/>} onPress={handleInstall}>Install</Button>
                )}
            </div>

            {/* Hero */}
            <div className="relative flex-shrink-0" style={{height: 200, background: "linear-gradient(180deg, rgba(8,9,10,0.3) 0%, rgba(8,9,10,0.85) 85%, var(--bg-0) 100%)", overflow: "hidden"}}>
                {item.gallery.length > 0 && <img src={item.gallery[0]} alt="" className="absolute inset-0 w-full h-full object-cover" style={{opacity: 0.3, filter: "blur(8px)"}}/>}
                <div className="absolute inset-0" style={{background: "linear-gradient(180deg, rgba(8,9,10,0.3) 0%, rgba(8,9,10,0.85) 85%, var(--bg-0) 100%)"}}/>
                <div className="relative flex items-end gap-5 h-full" style={{padding: "0 28px 24px"}}>
                    <div className="flex-shrink-0 rounded-2xl overflow-hidden flex items-center justify-center" style={{width: 90, height: 90, background: "var(--bg-1)", boxShadow: "0 20px 40px -10px rgba(0,0,0,0.8), 0 0 0 3px var(--bg-0)"}}>
                        {item.icon_url ? <img src={item.icon_url} alt={item.title} className="w-full h-full object-cover"/> : <I.box size={40} style={{color: "var(--ink-4)"}}/>}
                    </div>
                    <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1 flex-wrap">
                            <SourceBadge platform={item.platform}/>
                            {item.categories.slice(0, 3).map(c => <span key={c} style={{padding: "2px 8px", borderRadius: 999, fontSize: 10, fontWeight: 600, background: "rgba(255,255,255,0.06)", color: "var(--ink-2)"}}>{c}</span>)}
                        </div>
                        <h1 style={{fontSize: 28, fontWeight: 800, letterSpacing: -0.6, margin: 0}}>{item.title}</h1>
                        <div className="flex items-center gap-4 mt-1" style={{fontSize: 12, color: "var(--ink-3)"}}>
                            <span>{item.authors.map(a => a.name).join(", ") || "Unknown author"}</span>
                            <span className="flex items-center gap-1" style={{fontFamily: "var(--mono)"}}><I.download size={12}/> {formatCount(item.downloads)}</span>
                            <span className="flex items-center gap-1" style={{fontFamily: "var(--mono)"}}><I.heart size={12}/> {formatCount(item.follows)}</span>
                        </div>
                    </div>
                </div>
            </div>

            {/* Tabs */}
            <div className="flex border-b border-line" style={{padding: "0 28px"}}>
                {detailTabs.map(t => {
                    const isActive = t.key === tab;
                    const TabIcon = I[t.icon];
                    return (
                        <button key={t.key} onClick={() => setTab(t.key)} className="flex items-center gap-2 cursor-pointer bg-transparent"
                            style={{padding: "10px 16px", fontSize: 13, fontWeight: isActive ? 600 : 500,
                                color: isActive ? "var(--mc-green)" : "var(--ink-2)", border: "none",
                                borderBottom: isActive ? "2px solid var(--mc-green)" : "2px solid transparent"}}>
                            <TabIcon size={14}/> {t.label}
                        </button>
                    );
                })}
            </div>

            <div style={{display: "grid", gridTemplateColumns: "2fr 1fr", gap: 18, padding: "20px 28px 40px"}}>
                <div>
                    {tab === "summary" && <SummaryTab description={item.description} summary={item.summary}/>}
                    {tab === "gallery" && <GalleryTab images={item.gallery} title={item.title}/>}
                    {tab === "versions" && (
                        <InstanceVersionsTab
                            projectId={item.slug || item.id}
                            platform={item.platform.toLowerCase()}
                            instance={instance}
                            onInstalled={() => {
                                onInstalled(item.id);
                                setInstallState("installed");
                                // Refresh the installed record
                                invoke<InstalledModRecord[]>("get_installed_mods_info", {instanceId: instance.id})
                                    .then(records => {
                                        const match = records.find(r =>
                                            (r.modrinthId && (r.modrinthId === item.id || r.modrinthId === item.slug)) ||
                                            (r.curseforgeId && (r.curseforgeId === item.id || r.curseforgeId === item.slug))
                                        );
                                        if (match) setInstalledRecord(match);
                                    }).catch(() => {});
                            }}
                            currentVersionId={installedRecord?.versionId}
                            currentFileName={installedRecord?.fileName}
                            projectName={item.title}
                            onOpenDetail={navigateToMod}
                        />
                    )}
                    {tab === "dependencies" && (
                        <DependenciesTab
                            deps={deps}
                            projectId={item.slug || item.id}
                            platform={item.platform.toLowerCase()}
                            onOpenDetail={navigateToMod}
                        />
                    )}
                </div>
                <DetailSidebar item={item} loaders={loaders}/>
            </div>
        </div>
    );
}

// ---------------------------------------------------------------------------
// Versions tab with Install buttons per version
// ---------------------------------------------------------------------------

const versionTypeBadge: Record<string, { bg: string; color: string }> = {
    Release: {bg: "rgba(34,255,132,0.12)", color: "var(--mc-green)"},
    Beta: {bg: "rgba(255,180,50,0.12)", color: "#ffb432"},
    Alpha: {bg: "rgba(255,80,80,0.12)", color: "#ff5050"},
};

/** Resolves a project ID to a clickable name. Fetches the mod title lazily. */
function DepLink({projectId, platform, onClick}: { projectId: string; platform: string; onClick?: (item: ContentItem) => void }) {
    const [name, setName] = useState<string | null>(null);
    const [item, setItem] = useState<ContentItem | null>(null);

    useEffect(() => {
        invoke<ContentItem | null>("get_content", {id: projectId, platform, contentType: "mod"})
            .then(result => {
                if (result) {
                    setName(result.title);
                    setItem(result);
                }
            })
            .catch(() => {});
    }, [projectId, platform]);

    if (item && onClick) {
        return (
            <button
                className="text-mc-green hover:underline cursor-pointer bg-transparent border-none font-mono text-[11px] p-0"
                onClick={() => onClick(item)}
            >
                {name || projectId}
            </button>
        );
    }
    return <span style={{color: "var(--ink-2)", fontFamily: "var(--mono)"}}>{name || projectId}</span>;
}

function McVersionFilter({value, versions, onChange}: { value: string; versions: string[]; onChange: (v: string) => void }) {
    const [open, setOpen] = useState(false);
    const [search, setSearch] = useState("");
    const filtered = search
        ? versions.filter(v => v.includes(search))
        : versions;

    return (
        <Popover isOpen={open} onOpenChange={setOpen} placement="bottom-start">
            <PopoverTrigger>
                <Button variant="bordered" size="sm" endContent={<I.chevDown size={12}/>}
                    className="border-line bg-[rgba(0,0,0,0.3)] text-[var(--ink-1)] font-semibold text-xs font-mono">
                    {value || "All versions"}
                </Button>
            </PopoverTrigger>
            <PopoverContent className="p-0 bg-[#0d1117] border border-line w-[180px]">
                <div className="p-2 border-b border-line">
                    <input
                        autoFocus
                        placeholder="Search versions…"
                        value={search}
                        onChange={e => setSearch(e.target.value)}
                        className="w-full px-2.5 py-1.5 rounded-md border border-line bg-[rgba(0,0,0,0.3)] text-ink-0 text-xs font-mono outline-none"
                    />
                </div>
                <div className="overflow-y-auto max-h-[250px]">
                    <button
                        className={`w-full px-3 py-1.5 text-left text-xs cursor-pointer bg-transparent border-none font-sans transition-colors ${
                            !value ? "text-mc-green bg-[rgba(34,255,132,0.08)]" : "text-ink-2 hover:bg-[rgba(255,255,255,0.04)]"
                        }`}
                        onClick={() => { onChange(""); setOpen(false); setSearch(""); }}
                    >
                        All versions
                    </button>
                    {filtered.map(v => (
                        <button
                            key={v}
                            className={`w-full px-3 py-1.5 text-left text-xs font-mono cursor-pointer bg-transparent border-none transition-colors ${
                                value === v ? "text-mc-green bg-[rgba(34,255,132,0.08)]" : "text-ink-2 hover:bg-[rgba(255,255,255,0.04)]"
                            }`}
                            onClick={() => { onChange(v); setOpen(false); setSearch(""); }}
                        >
                            {v}
                        </button>
                    ))}
                    {filtered.length === 0 && (
                        <div className="px-3 py-3 text-xs text-ink-3 text-center">No versions match</div>
                    )}
                </div>
            </PopoverContent>
        </Popover>
    );
}

function InstanceVersionsTab({projectId, platform, instance, onInstalled, currentVersionId, currentFileName, projectName, onOpenDetail}: {
    projectId: string;
    platform: string;
    instance: Instance;
    onInstalled: () => void;
    currentVersionId?: string | null;
    currentFileName?: string | null;
    projectName?: string;
    onOpenDetail?: (item: ContentItem) => void;
}) {
    const [allVersions, setAllVersions] = useState<ProjectVersion[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [expandedId, setExpandedId] = useState<string | null>(null);
    const [installing, setInstalling] = useState<Set<string>>(new Set());
    const [installedVersions, setInstalledVersions] = useState<Set<string>>(new Set(currentVersionId ? [currentVersionId] : []));

    // Filters — default to instance's loader and MC version
    const [filterLoader, setFilterLoader] = useState<string>(instance.loader.toLowerCase());
    const [filterMcVersion, setFilterMcVersion] = useState<string>(instance.mc);
    const [filterType, setFilterType] = useState<string>("all");

    useEffect(() => {
        setLoading(true);
        invoke<ProjectVersion[]>("get_project_versions", {projectId, platform})
            .then(setAllVersions)
            .catch(e => setError(String(e)))
            .finally(() => setLoading(false));
    }, [projectId, platform]);

    // Derive available filter options from all versions
    const availableLoaders = [...new Set(allVersions.flatMap(v => v.loaders))].sort();
    const availableMcVersions = [...new Set(allVersions.flatMap(v => v.game_versions))];
    const availableTypes = [...new Set(allVersions.map(v => v.version_type))];

    // Apply filters
    const versions = allVersions.filter(v => {
        if (filterLoader && !v.loaders.some(l => l.toLowerCase() === filterLoader)) return false;
        if (filterMcVersion && !v.game_versions.includes(filterMcVersion)) return false;
        if (filterType !== "all" && v.version_type !== filterType) return false;
        return true;
    });

    const handleInstallVersion = async (v: ProjectVersion, replace: boolean) => {
        setInstalling(prev => new Set([...prev, v.id]));
        try {
            if (replace && currentFileName) {
                await invoke("replace_mod_version", {
                    instanceId: instance.id,
                    instancePath: instance.instancePath,
                    oldFileName: currentFileName,
                    versionId: v.id,
                    platform,
                    mcVersion: instance.mc,
                    loader: instance.loader.toLowerCase(),
                });
            } else {
                await invoke("install_mod_version", {
                    instanceId: instance.id,
                    instancePath: instance.instancePath,
                    versionId: v.id,
                    platform,
                    mcVersion: instance.mc,
                    loader: instance.loader.toLowerCase(),
                    projectName: projectName || null,
                });
            }
            setInstalledVersions(prev => new Set([...prev, v.id]));
            onInstalled();
        } catch (e) {
            console.error("Install version failed:", e);
        } finally {
            setInstalling(prev => { const n = new Set(prev); n.delete(v.id); return n; });
        }
    };

    if (loading) return <div className="flex items-center justify-center py-16"><Spinner size="lg" color="success"/></div>;
    if (error) return <div className="border border-line rounded-xl flex items-center gap-3" style={{...cardSurfaceStyle, padding: 16, borderColor: "rgba(255,80,80,0.2)", color: "#ff5050", fontSize: 12}}><I.x size={14}/> {error}</div>;
    if (allVersions.length === 0) return <div className="border border-line rounded-xl flex flex-col items-center justify-center gap-3" style={{...cardSurfaceStyle, padding: 48}}><I.pkg size={32} style={{color: "var(--ink-4)"}}/><span style={{fontSize: 13, color: "var(--ink-3)"}}>No versions available</span></div>;

    return (
        <div className="flex flex-col gap-3">
            {/* Filter bar */}
            <div className="flex items-center gap-2 flex-wrap">
                {/* Loader filter */}
                <Dropdown classNames={{content: "bg-[#0d1117] border border-line min-w-[140px]"}}>
                    <DropdownTrigger>
                        <Button variant="bordered" size="sm" endContent={<I.chevDown size={12}/>}
                            className="border-line bg-[rgba(0,0,0,0.3)] text-[var(--ink-1)] font-semibold text-xs">
                            {filterLoader || "All loaders"}
                        </Button>
                    </DropdownTrigger>
                    <DropdownMenu aria-label="Loader filter" selectionMode="single" selectedKeys={new Set([filterLoader])}
                        onSelectionChange={keys => { const v = [...keys][0] as string; setFilterLoader(v === "__all__" ? "" : v); }}
                        itemClasses={{base: "text-xs text-[var(--ink-2)] data-[hover=true]:bg-white/[0.04] data-[selected=true]:text-[var(--mc-green)]"}}>
                        <DropdownItem key="__all__">All loaders</DropdownItem>
                        {availableLoaders.map(l => <DropdownItem key={l.toLowerCase()}>{l}</DropdownItem>)}
                    </DropdownMenu>
                </Dropdown>

                {/* MC version filter — popover with search */}
                <McVersionFilter
                    value={filterMcVersion}
                    versions={availableMcVersions}
                    onChange={setFilterMcVersion}
                />

                {/* Release type filter */}
                <Dropdown classNames={{content: "bg-[#0d1117] border border-line min-w-[120px]"}}>
                    <DropdownTrigger>
                        <Button variant="bordered" size="sm" endContent={<I.chevDown size={12}/>}
                            className="border-line bg-[rgba(0,0,0,0.3)] text-[var(--ink-1)] font-semibold text-xs">
                            {filterType === "all" ? "All types" : filterType}
                        </Button>
                    </DropdownTrigger>
                    <DropdownMenu aria-label="Version type filter" selectionMode="single" selectedKeys={new Set([filterType])}
                        onSelectionChange={keys => { const v = [...keys][0] as string; setFilterType(v); }}
                        itemClasses={{base: "text-xs text-[var(--ink-2)] data-[hover=true]:bg-white/[0.04] data-[selected=true]:text-[var(--mc-green)]"}}>
                        <DropdownItem key="all">All types</DropdownItem>
                        {availableTypes.map(t => <DropdownItem key={t}>{t}</DropdownItem>)}
                    </DropdownMenu>
                </Dropdown>

                {(filterLoader || filterMcVersion || filterType !== "all") && (
                    <Button variant="light" size="sm" className="text-mc-green text-xs"
                        onPress={() => { setFilterLoader(""); setFilterMcVersion(""); setFilterType("all"); }}>
                        Clear filters
                    </Button>
                )}

                <div className="flex-1"/>
                <span className="text-xs text-ink-3 font-mono">
                    {versions.length} of {allVersions.length} version{allVersions.length !== 1 ? "s" : ""}
                </span>
            </div>

            {versions.length === 0 ? (
                <div className="border border-line rounded-xl flex flex-col items-center justify-center gap-3" style={{...cardSurfaceStyle, padding: 48}}>
                    <I.filter size={24} style={{color: "var(--ink-4)"}}/>
                    <span style={{fontSize: 13, color: "var(--ink-3)"}}>No versions match your filters</span>
                    <button onClick={() => { setFilterLoader(""); setFilterMcVersion(""); setFilterType("all"); }}
                        className="text-xs text-mc-green hover:underline cursor-pointer bg-transparent border-none font-sans">
                        Clear filters
                    </button>
                </div>
            ) : versions.map(v => {
                const badge = versionTypeBadge[v.version_type] ?? versionTypeBadge.Release;
                const primaryFile = v.files.find(f => f.primary) ?? v.files[0];
                const expanded = expandedId === v.id;
                const isInstalling = installing.has(v.id);
                const isInstalled = installedVersions.has(v.id);
                const isCurrentVersion = currentVersionId === v.id;
                const hasOtherVersionInstalled = !!currentVersionId && !isCurrentVersion && !isInstalled;

                return (
                    <div key={v.id} className="border border-line rounded-xl overflow-hidden" style={cardSurfaceStyle}>
                        {/* Header */}
                        <div className="flex items-center gap-3 cursor-pointer hover:bg-white/[0.02] transition-colors" style={{padding: "12px 16px"}} onClick={() => setExpandedId(expanded ? null : v.id)}>
                            <span style={{padding: "3px 8px", borderRadius: 6, fontSize: 10, fontWeight: 700, background: badge.bg, color: badge.color, flexShrink: 0}}>{v.version_type}</span>
                            <div className="flex-1 min-w-0">
                                <div style={{fontSize: 13, fontWeight: 600}} className="truncate">{v.name}</div>
                                <div className="flex items-center gap-3 mt-0.5" style={{fontSize: 11, color: "var(--ink-3)"}}>
                                    <span style={{fontFamily: "var(--mono)"}}>{v.version_number}</span>
                                    <span>{timeAgo(v.date_published)}</span>
                                    <span className="flex items-center gap-1"><I.download size={10}/> {formatCount(v.downloads)}</span>
                                </div>
                            </div>
                            <div className="flex items-center gap-1.5 flex-shrink-0 flex-wrap justify-end" style={{maxWidth: 200}}>
                                {v.loaders.slice(0, 3).map(l => <span key={l} style={{padding: "2px 7px", borderRadius: 4, fontSize: 9, fontWeight: 600, background: "rgba(255,255,255,0.06)", color: "var(--ink-2)"}}>{l}</span>)}
                                {v.game_versions.length > 0 && <span style={{padding: "2px 7px", borderRadius: 4, fontSize: 9, fontWeight: 600, background: "rgba(34,255,132,0.08)", color: "var(--mc-green)", fontFamily: "var(--mono)"}}>{v.game_versions[0]}{v.game_versions.length > 1 && ` +${v.game_versions.length - 1}`}</span>}
                            </div>

                            {/* Install / Replace button */}
                            <div onClick={e => e.stopPropagation()}>
                                {isCurrentVersion || isInstalled ? (
                                    <Button size="sm" variant="flat" className="font-bold text-mc-green" isDisabled startContent={<I.check size={12}/>}>Installed</Button>
                                ) : isInstalling ? (
                                    <Button size="sm" variant="bordered" isDisabled startContent={<div className="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"/>}>
                                        {hasOtherVersionInstalled ? "Replacing" : "Installing"}
                                    </Button>
                                ) : hasOtherVersionInstalled ? (
                                    <Button color="warning" size="sm" className="font-bold" startContent={<I.refresh size={12}/>} onPress={() => handleInstallVersion(v, true)}>Replace</Button>
                                ) : (
                                    <Button color="success" size="sm" className="font-bold" startContent={<I.download size={12}/>} onPress={() => handleInstallVersion(v, false)}>Install</Button>
                                )}
                            </div>

                            <I.chevDown size={14} style={{color: "var(--ink-3)", flexShrink: 0, transition: "transform 0.15s", transform: expanded ? "rotate(180deg)" : undefined}}/>
                        </div>

                        {/* Expanded */}
                        {expanded && (
                            <div style={{borderTop: "1px solid var(--line)", padding: "12px 16px"}}>
                                {v.game_versions.length > 1 && (
                                    <div className="mb-3">
                                        <div style={{fontSize: 10, fontWeight: 700, color: "var(--ink-3)", textTransform: "uppercase", letterSpacing: 1, marginBottom: 6}}>Game Versions</div>
                                        <div className="flex gap-1.5 flex-wrap">
                                            {v.game_versions.map(gv => <span key={gv} style={{padding: "2px 7px", borderRadius: 4, fontSize: 10, background: "rgba(255,255,255,0.04)", color: "var(--ink-2)", fontFamily: "var(--mono)"}}>{gv}</span>)}
                                        </div>
                                    </div>
                                )}
                                {v.files.length > 0 && (
                                    <div className="mb-3">
                                        <div style={{fontSize: 10, fontWeight: 700, color: "var(--ink-3)", textTransform: "uppercase", letterSpacing: 1, marginBottom: 6}}>Files</div>
                                        <div className="flex flex-col gap-1">
                                            {v.files.map((f, fi) => (
                                                <div key={fi} className="flex items-center gap-3" style={{fontSize: 11, padding: "4px 0"}}>
                                                    <I.pkg size={12} style={{color: "var(--ink-4)", flexShrink: 0}}/>
                                                    <span className="truncate flex-1" style={{color: "var(--ink-1)", fontFamily: "var(--mono)"}}>{f.filename}</span>
                                                    <span style={{color: "var(--ink-3)", fontSize: 10, fontFamily: "var(--mono)", flexShrink: 0}}>{formatSize(f.size)}</span>
                                                    {f.primary && <span style={{fontSize: 9, color: "var(--mc-green)", fontWeight: 600, flexShrink: 0}}>Primary</span>}
                                                </div>
                                            ))}
                                        </div>
                                    </div>
                                )}
                                {v.dependencies.length > 0 && (
                                    <div className="mb-3">
                                        <div style={{fontSize: 10, fontWeight: 700, color: "var(--ink-3)", textTransform: "uppercase", letterSpacing: 1, marginBottom: 6}}>Dependencies</div>
                                        <div className="flex flex-col gap-1">
                                            {v.dependencies.map((d, di) => (
                                                <div key={di} className="flex items-center gap-2" style={{fontSize: 11}}>
                                                    <span style={{padding: "1px 6px", borderRadius: 4, fontSize: 9, fontWeight: 700,
                                                        background: d.kind === "Required" ? "rgba(255,80,80,0.1)" : "rgba(255,255,255,0.04)",
                                                        color: d.kind === "Required" ? "#ff5050" : "var(--ink-3)"}}>{d.kind}</span>
                                                    {d.project_id ? (
                                                        <DepLink projectId={d.project_id} platform={platform} onClick={onOpenDetail}/>
                                                    ) : (
                                                        <span style={{color: "var(--ink-2)", fontFamily: "var(--mono)"}}>{d.version_id || "Unknown"}</span>
                                                    )}
                                                </div>
                                            ))}
                                        </div>
                                    </div>
                                )}
                                {v.changelog && (
                                    <div>
                                        <div style={{fontSize: 10, fontWeight: 700, color: "var(--ink-3)", textTransform: "uppercase", letterSpacing: 1, marginBottom: 6}}>Changelog</div>
                                        <div style={{fontSize: 12, lineHeight: 1.6, color: "var(--ink-2)", maxHeight: 200, overflowY: "auto", padding: 12, borderRadius: 8, background: "rgba(0,0,0,0.2)", whiteSpace: "pre-wrap", wordBreak: "break-word"}}>{v.changelog}</div>
                                    </div>
                                )}
                            </div>
                        )}
                    </div>
                );
            })}
        </div>
    );
}

// ---------------------------------------------------------------------------
// Card variants with Install button + onClick for detail
// ---------------------------------------------------------------------------

function InstallButton({installing, installed, onInstall}: { installing: boolean; installed: boolean; onInstall: () => void }) {
    if (installed) return <Button size="sm" variant="flat" className="font-bold text-mc-green" isDisabled startContent={<I.check size={12}/>}>Installed</Button>;
    if (installing) return <Button size="sm" variant="bordered" isDisabled startContent={<div className="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"/>}>Installing</Button>;
    return <Button color="success" size="sm" className="font-bold" startContent={<I.download size={12}/>} onPress={onInstall}>Install</Button>;
}

function InstallableCard({item, installing, installed, onInstall, onClick}: { item: ContentItem; installing: boolean; installed: boolean; onInstall: () => void; onClick: () => void }) {
    const loaders = "loaders" in item ? (item as { loaders: string[] }).loaders : [];
    return (
        <div className={`border border-line rounded-xl overflow-hidden cursor-pointer ${cardHoverClass}`} style={cardSurfaceStyle} onClick={onClick}>
            <div className="relative flex items-center justify-center" style={{height: 130, background: "rgba(0,0,0,0.25)", overflow: "hidden"}}>
                {item.icon_url ? <img src={item.icon_url} alt={item.title} className="w-full h-full object-cover"/> : <I.box size={40} style={{color: "var(--ink-4)"}}/>}
                {item.categories.length > 0 && <div className="absolute top-2 left-2 flex gap-1 flex-wrap">{item.categories.slice(0, 2).map(c => <span key={c} style={{fontSize: 9, padding: "2px 6px", borderRadius: 4, background: "rgba(0,0,0,0.5)", color: "var(--ink-2)", backdropFilter: "blur(6px)"}}>{c}</span>)}</div>}
                <div className="absolute bottom-2 left-2"><SourceBadge platform={item.platform}/></div>
            </div>
            <div style={{padding: "12px 14px"}}>
                <div style={{fontSize: 13, fontWeight: 700, marginBottom: 2}} className="truncate">{item.title}</div>
                <div style={{fontSize: 10, color: "var(--ink-3)"}} className="truncate">{item.authors?.map(a => a.name).join(", ") || "Unknown"}</div>
                <div style={{fontSize: 11, color: "var(--ink-2)", lineHeight: 1.45, minHeight: 32, marginTop: 4, display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical", overflow: "hidden"}}>{item.summary}</div>
                <div className="flex items-center gap-2 mt-3" onClick={e => e.stopPropagation()}>
                    <span className="flex items-center gap-1 text-ink-3 font-mono" style={{fontSize: 11}}><I.download size={11}/> {formatCount(item.downloads)}</span>
                    <div className="flex-1"/>
                    <InstallButton installing={installing} installed={installed} onInstall={onInstall}/>
                </div>
            </div>
        </div>
    );
}

function InstallableCompact({item, installing, installed, onInstall, onClick}: { item: ContentItem; installing: boolean; installed: boolean; onInstall: () => void; onClick: () => void }) {
    return (
        <div className={`border border-line rounded-xl px-3.5 py-3 flex items-center gap-3 cursor-pointer ${cardHoverClass}`} style={cardSurfaceStyle} onClick={onClick}>
            {item.icon_url ? <img src={item.icon_url} alt="" className="w-9 h-9 rounded-lg flex-shrink-0 object-cover"/> : <div className="w-9 h-9 rounded-lg flex-shrink-0 flex items-center justify-center border border-line bg-[rgba(255,255,255,0.04)] text-ink-3"><I.box size={16}/></div>}
            <div className="min-w-0 flex-1"><div className="text-xs font-bold truncate">{item.title}</div><div className="text-[0.625rem] text-ink-3 truncate">{item.summary}</div></div>
            <div onClick={e => e.stopPropagation()}><InstallButton installing={installing} installed={installed} onInstall={onInstall}/></div>
        </div>
    );
}

function InstallableRow({item, isLast, installing, installed, onInstall, onClick}: { item: ContentItem; isLast: boolean; installing: boolean; installed: boolean; onInstall: () => void; onClick: () => void }) {
    return (
        <div className={`flex items-center gap-3.5 px-4 py-3 transition-colors hover:bg-[rgba(255,255,255,0.02)] cursor-pointer ${isLast ? "" : "border-b border-line"}`} onClick={onClick}>
            {item.icon_url ? <img src={item.icon_url} alt="" className="w-9 h-9 rounded-lg flex-shrink-0 object-cover"/> : <div className="w-9 h-9 rounded-lg flex-shrink-0 flex items-center justify-center border border-line bg-[rgba(255,255,255,0.04)] text-ink-3"><I.box size={16}/></div>}
            <div className="min-w-0 flex-1"><div className="text-[0.8125rem] font-semibold truncate">{item.title}</div><div className="text-[0.6875rem] text-ink-3 mt-0.5 truncate">{item.summary}</div></div>
            <span className="text-ink-3 font-mono text-[0.6875rem] flex items-center gap-1 flex-shrink-0"><I.download size={10}/> {formatCount(item.downloads)}</span>
            <span className="text-ink-4 font-mono text-[0.6875rem] flex-shrink-0">{timeAgo(item.updated)}</span>
            <div onClick={e => e.stopPropagation()}><InstallButton installing={installing} installed={installed} onInstall={onInstall}/></div>
        </div>
    );
}
