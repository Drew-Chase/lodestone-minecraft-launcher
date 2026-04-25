import {useState, useCallback} from "react";
import {Outlet, useMatch} from "react-router-dom";
import {Input, Spinner} from "@heroui/react";
import TitleBar from "../components/shell/TitleBar";
import {I} from "../components/shell/icons";
import ContentTabs from "../components/discover/ContentTabs";
import SourceTabs, {isSourceAvailable, getSourcesForContentType} from "../components/discover/SourceTabs";
import SortSelect from "../components/discover/SortSelect";
import ViewModeToggle from "../components/discover/ViewModeToggle";
import FilterDrawer from "../components/discover/FilterDrawer";
import {isFilterKeyRelevant} from "../components/discover/filterConfig";
import BrowseGrid from "../components/discover/BrowseGrid";
import BrowseCompact from "../components/discover/BrowseCompact";
import BrowseTable from "../components/discover/BrowseTable";
import useContentSearch from "../hooks/useContentSearch";
import {usePersistedState} from "../hooks/usePersistedState";
import type {ContentTypeKey, FilterState, SortKey, SourceKey, ViewMode} from "../types/content";
import {defaultFilterState} from "../types/content";

export default function Discover() {
    // When a child route is active (/discover/:platform/:id), we render it
    // as an overlay so the Discover page stays mounted (scroll, results, query preserved).
    const detailMatch = useMatch("/discover/:platform/:id");
    const showDetail = !!detailMatch;
    const [contentType, setContentTypePersisted] = usePersistedState<ContentTypeKey>("discover.contentType", "mod");
    const [source, setSource] = usePersistedState<SourceKey>("discover.source", "modrinth");
    const [query, setQuery] = useState("");
    const [sort, setSort] = useState<SortKey>("downloads");
    const [viewMode, setViewMode] = usePersistedState<ViewMode>("discover.viewMode", "grid");
    const [showFilter, setShowFilter] = useState(false);
    const [filters, setFilters] = usePersistedState<FilterState>("discover.filters", defaultFilterState);

    const setContentType = useCallback((ct: ContentTypeKey) => {
        setContentTypePersisted(ct);
        if (!isSourceAvailable(source, ct)) {
            const first = getSourcesForContentType(ct).find(s => !s.disabled);
            if (first) setSource(first.key);
        }
        // Clear filter fields that aren't relevant for the new content type.
        const cleaned = {...filters};
        for (const key of Object.keys(cleaned) as (keyof FilterState)[]) {
            if (!isFilterKeyRelevant(key, ct)) {
                cleaned[key] = [];
            }
        }
        setFilters(cleaned);
    }, [source, filters, setContentTypePersisted, setSource, setFilters]);

    const {results, loading, error, hasMore, sentinelRef} = useContentSearch({
        query,
        sort,
        source,
        contentType,
        filters,
    });

    const activeFilterCount =
        filters.categories.length + filters.loaders.length +
        filters.versions.length + filters.environment.length;

    return (
        <div className="flex-1 flex flex-col overflow-hidden relative" style={{background: "var(--bg-0)"}}>
            {/* ── Static header ── */}
            <div className="flex-shrink-0">
                <TitleBar title="Discover" subtitle="Browse mods, modpacks, shaders, and more"/>

                {/* Content type tabs */}
                <ContentTabs active={contentType} onChange={setContentType}/>

                {/* Filter + search toolbar */}
                <div
                    className="flex items-center gap-3 flex-wrap relative border-b border-line"
                    style={{padding: "14px 28px"}}
                >
                    {/* Search */}
                    <div className="relative" style={{width: 340}}>
                        <Input
                            placeholder="Search mods, modpacks..."
                            value={query}
                            onValueChange={setQuery}
                            startContent={<I.search size={14} style={{color: "var(--ink-3)"}}/>}
                            size="sm"
                            variant="bordered"
                            classNames={{
                                inputWrapper: "border-line bg-[rgba(0,0,0,0.3)]",
                            }}
                        />
                    </div>

                    {/* Source */}
                    <SourceTabs active={source} contentType={contentType} onChange={setSource}/>

                    <div className="flex-1"/>

                    {/* Sort */}
                    <SortSelect value={sort} onChange={setSort}/>

                    {/* Filter button */}
                    <button
                        onClick={() => setShowFilter(true)}
                        className="cursor-pointer flex items-center gap-2"
                        style={{
                            padding: "7px 12px",
                            fontSize: 12,
                            fontWeight: 600,
                            borderRadius: 8,
                            border: activeFilterCount > 0
                                ? "1px solid rgba(34,255,132,0.4)"
                                : "1px solid var(--line)",
                            background: activeFilterCount > 0
                                ? "rgba(34,255,132,0.08)"
                                : "transparent",
                            color: activeFilterCount > 0 ? "var(--mc-green)" : "var(--ink-2)",
                            transition: "all 0.12s",
                        }}
                    >
                        <I.filter size={14}/>
                        Filters
                        {activeFilterCount > 0 && (
                            <span
                                style={{
                                    padding: "1px 6px",
                                    borderRadius: 999,
                                    fontSize: 10,
                                    fontWeight: 700,
                                    background: "rgba(34,255,132,0.14)",
                                    color: "var(--mc-green)",
                                }}
                            >
                                {activeFilterCount}
                            </span>
                        )}
                    </button>

                    {/* View mode */}
                    <ViewModeToggle value={viewMode} onChange={setViewMode}/>

                    {/* Result count */}
                    <span style={{fontSize: 12, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                        {results.length} result{results.length !== 1 ? "s" : ""}
                    </span>
                </div>
            </div>

            {/* ── Scrollable results ── */}
            <div
                className={viewMode === "table"
                    ? "flex-1 flex flex-col min-h-0 overflow-hidden"
                    : "flex-1 overflow-y-auto"
                }
                style={{padding: viewMode === "table" ? "20px 28px 0" : "20px 28px 40px"}}
            >
                {error && (
                    <div
                        className="border border-line rounded-xl flex items-center gap-3 flex-shrink-0"
                        style={{
                            padding: 16,
                            marginBottom: 14,
                            background: "rgba(255,80,80,0.06)",
                            borderColor: "rgba(255,80,80,0.2)",
                            color: "#ff5050",
                            fontSize: 12,
                        }}
                    >
                        <I.x size={14}/>
                        {error}
                    </div>
                )}

                {loading && results.length === 0 ? (
                    <div className="flex items-center justify-center py-20">
                        <Spinner size="lg" color="success"/>
                    </div>
                ) : results.length === 0 && !loading ? (
                    <div className="flex flex-col items-center justify-center py-20 gap-3">
                        <I.search size={40} style={{color: "var(--ink-4)"}}/>
                        <span style={{fontSize: 14, color: "var(--ink-3)"}}>
                            No results found
                        </span>
                        <span style={{fontSize: 12, color: "var(--ink-4)"}}>
                            Try a different query or adjust your filters
                        </span>
                    </div>
                ) : (
                    <>
                        {viewMode === "grid" && <BrowseGrid items={results}/>}
                        {viewMode === "compact" && <BrowseCompact items={results}/>}
                        {viewMode === "table" && (
                            <BrowseTable
                                items={results}
                                hasMore={hasMore}
                                loading={loading}
                                sentinelRef={sentinelRef}
                            />
                        )}

                        {viewMode !== "table" && (
                            <>
                                {hasMore && (
                                    <div
                                        ref={sentinelRef}
                                        className="flex items-center justify-center py-6"
                                    >
                                        {loading && <Spinner size="sm" color="success"/>}
                                    </div>
                                )}

                                {!hasMore && results.length > 0 && (
                                    <div
                                        className="flex items-center justify-center py-6"
                                        style={{fontSize: 12, color: "var(--ink-4)", fontFamily: "var(--mono)"}}
                                    >
                                        End of results
                                    </div>
                                )}
                            </>
                        )}
                    </>
                )}
            </div>

            <FilterDrawer
                isOpen={showFilter}
                onClose={() => setShowFilter(false)}
                filters={filters}
                onChange={setFilters}
                contentType={contentType}
                source={source}
            />

            {/* Content detail overlay — Discover stays mounted underneath */}
            {showDetail && (
                <div className="absolute inset-0 z-30 flex flex-col" style={{background: "var(--bg-0)"}}>
                    <Outlet/>
                </div>
            )}
        </div>
    );
}
