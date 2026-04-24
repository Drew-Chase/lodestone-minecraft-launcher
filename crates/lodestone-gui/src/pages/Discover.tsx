import {useState} from "react";
import {Input, Spinner} from "@heroui/react";
import TitleBar from "../components/shell/TitleBar";
import {I} from "../components/shell/icons";
import ContentTabs from "../components/discover/ContentTabs";
import SourceTabs from "../components/discover/SourceTabs";
import SortSelect from "../components/discover/SortSelect";
import ViewModeToggle from "../components/discover/ViewModeToggle";
import FilterPopover, {type FilterState} from "../components/discover/FilterPopover";
import BrowseGrid from "../components/discover/BrowseGrid";
import BrowseCompact from "../components/discover/BrowseCompact";
import BrowseTable from "../components/discover/BrowseTable";
import useContentSearch from "../hooks/useContentSearch";
import type {ContentTypeKey, SortKey, SourceKey, ViewMode} from "../types/content";

export default function Discover() {
    const [contentType, setContentType] = useState<ContentTypeKey>("mod");
    const [source, setSource] = useState<SourceKey>("modrinth");
    const [query, setQuery] = useState("");
    const [sort, setSort] = useState<SortKey>("downloads");
    const [viewMode, setViewMode] = useState<ViewMode>("grid");
    const [showFilter, setShowFilter] = useState(false);
    const [filters, setFilters] = useState<FilterState>({
        categories: [],
        loaders: [],
        versions: [],
        environment: [],
    });

    const {results, loading, error, hasMore, sentinelRef} = useContentSearch({
        query,
        sort,
        source,
        contentType,
    });

    const activeFilterCount =
        filters.categories.length + filters.loaders.length +
        filters.versions.length + filters.environment.length;

    return (
        <div className="flex-1 flex flex-col overflow-hidden" style={{background: "var(--bg-0)"}}>
            {/* ── Static header ── */}
            <div className="flex-shrink-0">
                <TitleBar title="Discover" subtitle="Browse mods, modpacks, shaders, and more"/>

                {/* Content type tabs */}
                <ContentTabs active={contentType} onChange={ct => {setContentType(ct); setSource("modrinth");}}/>

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
                    <div className="relative">
                        <button
                            onClick={() => setShowFilter(v => !v)}
                            className="cursor-pointer flex items-center gap-2"
                            style={{
                                padding: "7px 12px",
                                fontSize: 12,
                                fontWeight: 600,
                                borderRadius: 8,
                                border: showFilter
                                    ? "1px solid rgba(34,255,132,0.4)"
                                    : "1px solid var(--line)",
                                background: showFilter
                                    ? "rgba(34,255,132,0.08)"
                                    : "transparent",
                                color: showFilter ? "var(--mc-green)" : "var(--ink-2)",
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
                        {showFilter && (
                            <FilterPopover
                                filters={filters}
                                onChange={setFilters}
                                onClose={() => setShowFilter(false)}
                            />
                        )}
                    </div>

                    {/* View mode */}
                    <ViewModeToggle value={viewMode} onChange={setViewMode}/>

                    {/* Result count */}
                    <span style={{fontSize: 12, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                        {results.length} result{results.length !== 1 ? "s" : ""}
                    </span>
                </div>
            </div>

            {/* ── Scrollable results ── */}
            <div className="flex-1 overflow-y-auto" style={{padding: "20px 28px 40px"}}>
                {error && (
                    <div
                        className="border border-line rounded-xl flex items-center gap-3"
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
                        {viewMode === "table" && <BrowseTable items={results}/>}

                        {/* Infinite-scroll sentinel: when this scrolls into
                            view (or within 200px of it), the hook fires the
                            next page request automatically. */}
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
            </div>
        </div>
    );
}
