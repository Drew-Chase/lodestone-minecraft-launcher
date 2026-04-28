import {useState, useCallback, useRef, useEffect} from "react";
import {useNavigate} from "react-router-dom";
import {Button, Input, Spinner, Tooltip} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {I} from "../shell/icons";
import {cardSurfaceStyle, cardHoverClass} from "../surfaces";
import ContentTabs from "./ContentTabs";
import SourceTabs, {isSourceAvailable, getSourcesForContentType} from "./SourceTabs";
import SortSelect from "./SortSelect";
import ViewModeToggle from "./ViewModeToggle";
import FilterDrawer from "./FilterDrawer";
import {isFilterKeyRelevant} from "./filterConfig";
import SourceBadge from "./SourceBadge";
import BrowseCard, {InstallBtn} from "./BrowseCard";
import useContentSearch from "../../hooks/useContentSearch";
import {usePersistedState} from "../../hooks/usePersistedState";
import type {ContentItem, ContentTypeKey, FilterState, SortKey, SourceKey, ViewMode} from "../../types/content";
import {defaultFilterState, formatCount, timeAgo} from "../../types/content";

export interface ContentBrowserProps {
    /** If set, instance mode: shows install buttons, hides modpacks tab. */
    instanceId?: number;
    instancePath?: string;
    mcVersion?: string;
    loader?: string;
    /** Back button handler (shown in instance mode). */
    onBack?: () => void;
    /** Called when an item is clicked in discover mode (no instance). Defaults to navigate. */
    onItemClick?: (item: ContentItem) => void;
}

export default function ContentBrowser({
    instanceId, instancePath, mcVersion, loader, onBack, onItemClick,
}: ContentBrowserProps) {
    const navigate = useNavigate();
    const isInstanceMode = instanceId != null;
    const persistPrefix = isInstanceMode ? "modBrowser" : "discover";

    const defaultContentType: ContentTypeKey = isInstanceMode ? "mod" : "modpack";
    const [contentType, setContentTypePersisted] = usePersistedState<ContentTypeKey>(
        `${persistPrefix}.contentType`, defaultContentType,
    );
    const [source, setSource] = usePersistedState<SourceKey>(`${persistPrefix}.source`, "modrinth");
    const [query, setQuery] = useState("");
    const [sort, setSort] = useState<SortKey>("downloads");
    const [viewMode, setViewMode] = usePersistedState<ViewMode>(`${persistPrefix}.viewMode`, "grid");
    const [showFilter, setShowFilter] = useState(false);
    const [filters, setFilters] = usePersistedState<FilterState>(`${persistPrefix}.filters`, {
        ...defaultFilterState,
        ...(isInstanceMode && loader ? {loaders: [loader.toLowerCase()]} : {}),
        ...(isInstanceMode && mcVersion ? {versions: [mcVersion]} : {}),
    });

    // Install state (instance mode only)
    const [installing, setInstalling] = useState<Set<string>>(new Set());
    const [installed, setInstalled] = useState<Set<string>>(new Set());

    const setContentType = useCallback((ct: ContentTypeKey) => {
        setContentTypePersisted(ct);
        if (!isSourceAvailable(source, ct)) {
            const first = getSourcesForContentType(ct).find(s => !s.disabled);
            if (first) setSource(first.key);
        }
        const cleaned = {...filters};
        for (const key of Object.keys(cleaned) as (keyof FilterState)[]) {
            if (!isFilterKeyRelevant(key, ct)) {
                cleaned[key] = [];
            }
        }
        setFilters(cleaned);
    }, [source, filters, setContentTypePersisted, setSource, setFilters]);

    const {results, loading, error, hasMore, sentinelRef} = useContentSearch({
        query, sort, source, contentType, filters,
    });

    const activeFilterCount =
        filters.categories.length + filters.loaders.length +
        filters.versions.length + filters.environment.length;

    // Install handler (instance mode)
    const handleInstall = useCallback(async (item: ContentItem) => {
        if (!instanceId || !instancePath || !mcVersion || !loader) return;
        setInstalling(prev => new Set([...prev, item.id]));
        try {
            if (contentType === "modpack") {
                // For modpacks, fetch latest version and install
                const versions = await invoke<{id: string}[]>("get_project_versions", {
                    projectId: item.id,
                    platform: item.platform.toLowerCase(),
                });
                if (versions?.length > 0) {
                    await invoke("install_modpack_from_discover", {
                        projectId: item.id,
                        versionId: versions[0].id,
                        platform: item.platform.toLowerCase(),
                    });
                }
            } else {
                await invoke("install_mod", {
                    instanceId, instancePath, projectId: item.id,
                    platform: item.platform.toLowerCase(), mcVersion, loader: loader.toLowerCase(),
                    projectName: item.title,
                });
            }
            setInstalled(prev => new Set([...prev, item.id]));
        } catch (e) {
            console.error("Install failed:", e);
        } finally {
            setInstalling(prev => { const n = new Set(prev); n.delete(item.id); return n; });
        }
    }, [instanceId, instancePath, mcVersion, loader, contentType]);

    const handleItemClick = useCallback((item: ContentItem) => {
        if (onItemClick) {
            onItemClick(item);
        } else {
            navigate(`/discover/${item.platform.toLowerCase()}/${item.slug || item.id}`);
        }
    }, [navigate, onItemClick]);

    return (
        <div className="flex-1 flex flex-col overflow-hidden relative">
            {/* Header */}
            <div className="flex-shrink-0">
                <ContentTabs active={contentType} onChange={setContentType} hideModpacks={isInstanceMode}/>

                {/* Toolbar */}
                <div className="flex items-center gap-3 flex-wrap border-b border-line" style={{padding: "14px 28px"}}>
                    {onBack && (
                        <Button variant="bordered" size="sm" startContent={<I.chevRight size={13} className="rotate-180"/>} onPress={onBack}>
                            Back
                        </Button>
                    )}
                    <div className="relative" style={{width: isInstanceMode ? 300 : 340}}>
                        <Input
                            placeholder="Search mods, modpacks..."
                            value={query}
                            onValueChange={setQuery}
                            startContent={<I.search size={14} style={{color: "var(--ink-3)"}}/>}
                            size="sm"
                            variant="bordered"
                            classNames={{inputWrapper: "border-line bg-[rgba(0,0,0,0.3)]"}}
                        />
                    </div>
                    <SourceTabs active={source} contentType={contentType} onChange={setSource}/>
                    <div className="flex-1"/>
                    <SortSelect value={sort} onChange={setSort}/>
                    <button
                        onClick={() => setShowFilter(true)}
                        className="cursor-pointer flex items-center gap-2"
                        style={{
                            padding: "7px 12px", fontSize: 12, fontWeight: 600, borderRadius: 8,
                            border: activeFilterCount > 0 ? "1px solid rgba(34,255,132,0.4)" : "1px solid var(--line)",
                            background: activeFilterCount > 0 ? "rgba(34,255,132,0.08)" : "transparent",
                            color: activeFilterCount > 0 ? "var(--mc-green)" : "var(--ink-2)",
                        }}
                    >
                        <I.filter size={14}/> Filters
                        {activeFilterCount > 0 && (
                            <span style={{padding: "1px 6px", borderRadius: 999, fontSize: 10, fontWeight: 700, background: "rgba(34,255,132,0.14)", color: "var(--mc-green)"}}>
                                {activeFilterCount}
                            </span>
                        )}
                    </button>
                    <ViewModeToggle value={viewMode} onChange={setViewMode}/>
                    <span style={{fontSize: 12, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                        {results.length} result{results.length !== 1 ? "s" : ""}
                    </span>
                </div>
            </div>

            {/* Results */}
            <div className="flex-1 overflow-y-auto" style={{padding: "20px 28px 40px"}}>
                {error && (
                    <div className="border border-line rounded-xl flex items-center gap-3 mb-3.5" style={{padding: 16, background: "rgba(255,80,80,0.06)", borderColor: "rgba(255,80,80,0.2)", color: "#ff5050", fontSize: 12}}>
                        <I.x size={14}/> {error}
                    </div>
                )}
                {loading && results.length === 0 ? (
                    <div className="flex items-center justify-center py-20"><Spinner size="lg" color="success"/></div>
                ) : results.length === 0 && !loading ? (
                    <div className="flex flex-col items-center justify-center py-20 gap-3">
                        <I.search size={40} style={{color: "var(--ink-4)"}}/>
                        <span style={{fontSize: 14, color: "var(--ink-3)"}}>No results found</span>
                        <span style={{fontSize: 12, color: "var(--ink-4)"}}>Try a different query or adjust your filters</span>
                    </div>
                ) : (
                    <>
                        {viewMode === "grid" && (
                            <div style={{display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))", gap: 14}}>
                                {results.map((item, i) => (
                                    <BrowseCard
                                        key={`${item.id}-${i}`}
                                        item={item}
                                        onClick={() => handleItemClick(item)}
                                        onInstall={isInstanceMode ? () => handleInstall(item) : undefined}
                                        installing={installing.has(item.id)}
                                        installed={installed.has(item.id)}
                                    />
                                ))}
                            </div>
                        )}
                        {viewMode === "compact" && (
                            <div style={{display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: 10}}>
                                {results.map((item, i) => (
                                    <CompactRow
                                        key={`${item.id}-${i}`}
                                        item={item}
                                        onClick={() => handleItemClick(item)}
                                        onInstall={isInstanceMode ? () => handleInstall(item) : undefined}
                                        installing={installing.has(item.id)}
                                        installed={installed.has(item.id)}
                                    />
                                ))}
                            </div>
                        )}
                        {viewMode === "table" && (
                            <div className="rounded-xl border border-line overflow-hidden" style={cardSurfaceStyle}>
                                {results.map((item, i) => (
                                    <ListRow
                                        key={`${item.id}-${i}`}
                                        item={item}
                                        isLast={i === results.length - 1}
                                        onClick={() => handleItemClick(item)}
                                        onInstall={isInstanceMode ? () => handleInstall(item) : undefined}
                                        installing={installing.has(item.id)}
                                        installed={installed.has(item.id)}
                                    />
                                ))}
                            </div>
                        )}
                        {hasMore && (
                            <div ref={sentinelRef} className="flex items-center justify-center py-6">
                                {loading && <Spinner size="sm" color="success"/>}
                            </div>
                        )}
                        {!hasMore && results.length > 0 && (
                            <div className="flex items-center justify-center py-6" style={{fontSize: 12, color: "var(--ink-4)", fontFamily: "var(--mono)"}}>
                                End of results
                            </div>
                        )}
                    </>
                )}
            </div>

            <FilterDrawer isOpen={showFilter} onClose={() => setShowFilter(false)} filters={filters} onChange={setFilters} contentType={contentType} source={source}/>
        </div>
    );
}

// ---------------------------------------------------------------------------
// Compact row (icon + title + summary + optional install)
// ---------------------------------------------------------------------------

function CompactRow({item, onClick, onInstall, installing, installed}: {
    item: ContentItem; onClick: () => void;
    onInstall?: () => void; installing?: boolean; installed?: boolean;
}) {
    const titleRef = useRef<HTMLSpanElement>(null);
    const [isTruncated, setIsTruncated] = useState(false);
    useEffect(() => {
        const el = titleRef.current;
        if (el) setIsTruncated(el.scrollWidth > el.clientWidth);
    }, [item.title]);

    const titleEl = (
        <span ref={titleRef} style={{fontSize: 12, fontWeight: 700}} className="truncate flex-1 min-w-0">
            {item.title}
        </span>
    );

    return (
        <div
            className={`border border-line rounded-xl overflow-hidden cursor-pointer flex ${cardHoverClass}`}
            style={{...cardSurfaceStyle, padding: 0}}
            onClick={onClick}
        >
            <div className="flex-shrink-0 flex items-center justify-center" style={{width: 64, background: "rgba(0,0,0,0.25)"}}>
                {item.icon_url ? (
                    <img src={item.icon_url} alt={item.title} className="w-full h-full object-cover"/>
                ) : (
                    <I.box size={22} style={{color: "var(--ink-4)"}}/>
                )}
            </div>
            <div style={{flex: 1, padding: "6px 10px", minWidth: 0}}>
                <div className="flex items-center gap-1.5">
                    {isTruncated ? <Tooltip content={item.title} delay={400} classNames={{content: "bg-[#0d1117] border border-line text-xs text-[var(--ink-1)] max-w-[300px]"}}>{titleEl}</Tooltip> : titleEl}
                    <SourceBadge platform={item.platform}/>
                </div>
                <div style={{fontSize: 9, color: "var(--ink-3)"}} className="truncate">
                    {item.authors?.map(a => a.name).join(", ") || "Unknown"}
                </div>
                <div style={{fontSize: 10, color: "var(--ink-2)", lineHeight: 1.4, marginTop: 1, display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical", overflow: "hidden"}}>
                    {item.summary}
                </div>
                <div className="flex items-center gap-2 mt-0.5" style={{fontSize: 9, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                    <span className="flex items-center gap-1"><I.download size={9}/> {formatCount(item.downloads)}</span>
                    {onInstall && (
                        <div className="ml-auto" onClick={e => e.stopPropagation()}>
                            <InstallBtn installing={installing} installed={installed} onInstall={onInstall}/>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}

// ---------------------------------------------------------------------------
// List row (vertical list, NOT a table — matches instance browser style)
// ---------------------------------------------------------------------------

function ListRow({item, isLast, onClick, onInstall, installing, installed}: {
    item: ContentItem; isLast: boolean; onClick: () => void;
    onInstall?: () => void; installing?: boolean; installed?: boolean;
}) {
    return (
        <div
            className={`flex items-center gap-3.5 px-4 py-3 transition-colors hover:bg-[rgba(255,255,255,0.02)] cursor-pointer ${isLast ? "" : "border-b border-line"}`}
            onClick={onClick}
        >
            {item.icon_url ? (
                <img src={item.icon_url} alt="" className="w-9 h-9 rounded-lg flex-shrink-0 object-cover"/>
            ) : (
                <div className="w-9 h-9 rounded-lg flex-shrink-0 flex items-center justify-center border border-line bg-[rgba(255,255,255,0.04)] text-ink-3">
                    <I.box size={16}/>
                </div>
            )}
            <div className="min-w-0 flex-1">
                <div className="text-[0.8125rem] font-semibold truncate">{item.title}</div>
                <div className="text-[0.6875rem] text-ink-3 mt-0.5 truncate">{item.summary}</div>
            </div>
            <SourceBadge platform={item.platform}/>
            <span className="text-ink-3 font-mono text-[0.6875rem] flex items-center gap-1 flex-shrink-0">
                <I.download size={10}/> {formatCount(item.downloads)}
            </span>
            <span className="text-ink-4 font-mono text-[0.6875rem] flex-shrink-0">{timeAgo(item.updated)}</span>
            {onInstall && (
                <div onClick={e => e.stopPropagation()}>
                    <InstallBtn installing={installing} installed={installed} onInstall={onInstall}/>
                </div>
            )}
        </div>
    );
}
