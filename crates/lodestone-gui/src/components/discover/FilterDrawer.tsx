import {useState} from "react";
import {Drawer, DrawerContent, DrawerHeader, DrawerBody, DrawerFooter} from "@heroui/drawer";
import {Input} from "@heroui/react";
import {I} from "../shell/icons";
import {useMinecraftVersions} from "../../hooks/useMinecraftVersions";
import {getFilterSections, type FilterSectionDef} from "./filterConfig";
import type {ContentTypeKey, FilterState, SourceKey} from "../../types/content";
import {defaultFilterState} from "../../types/content";

interface FilterDrawerProps {
    isOpen: boolean;
    onClose: () => void;
    filters: FilterState;
    onChange: (f: FilterState) => void;
    contentType: ContentTypeKey;
    source: SourceKey;
}

const VERSION_PAGE_SIZE = 30;

export default function FilterDrawer({
    isOpen,
    onClose,
    filters,
    onChange,
    contentType,
    source,
}: FilterDrawerProps) {
    const sections = getFilterSections(contentType, source);
    const {versions} = useMinecraftVersions();

    const [versionSearch, setVersionSearch] = useState("");
    const [showSnapshots, setShowSnapshots] = useState(false);
    const [versionLimit, setVersionLimit] = useState(VERSION_PAGE_SIZE);

    const toggle = (arr: string[], item: string): string[] =>
        arr.includes(item) ? arr.filter(x => x !== item) : [...arr, item];

    const activeCount =
        filters.categories.length + filters.loaders.length +
        filters.versions.length + filters.environment.length;

    // Filter and slice versions for display.
    const filteredVersions = versions.filter(v => {
        if (!showSnapshots && v.version_type !== "Release") return false;
        return !(versionSearch && !v.id.includes(versionSearch));

    });
    const visibleVersions = filteredVersions.slice(0, versionLimit);
    const hasMoreVersions = filteredVersions.length > versionLimit;

    return (
        <Drawer
            isOpen={isOpen}
            onClose={onClose}
            hideCloseButton
            placement="right"
            backdrop="blur"
            classNames={{
                base: "bg-[#0a0d10] border-l border-line max-w-[420px] w-[420px]",
                header: "border-b border-line px-6 py-4",
                body: "px-6 py-4",
                footer: "border-t border-line px-6 py-3",
            }}
        >
            <DrawerContent>
                <DrawerHeader>
                    <div className="flex items-center gap-3 w-full">
                        <I.filter size={16} style={{color: "var(--mc-green)"}}/>
                        <span className="text-sm font-bold flex-1">Refine results</span>
                        {activeCount > 0 && (
                            <span
                                style={{
                                    padding: "2px 8px",
                                    borderRadius: 999,
                                    background: "rgba(34,255,132,0.14)",
                                    color: "var(--mc-green)",
                                    fontSize: 10,
                                    fontWeight: 700,
                                }}
                            >
                                {activeCount}
                            </span>
                        )}
                    </div>
                </DrawerHeader>

                <DrawerBody>
                    <div className="flex flex-col gap-5">
                        {sections.map(section =>
                            section.searchable ? (
                                <VersionSection
                                    key={section.key}
                                    section={section}
                                    selected={filters[section.key]}
                                    onToggle={item =>
                                        onChange({...filters, [section.key]: toggle(filters[section.key], item)})
                                    }
                                    versions={visibleVersions.map(v => v.id)}
                                    versionSearch={versionSearch}
                                    onSearchChange={v => {
                                        setVersionSearch(v);
                                        setVersionLimit(VERSION_PAGE_SIZE);
                                    }}
                                    showSnapshots={showSnapshots}
                                    onToggleSnapshots={() => setShowSnapshots(v => !v)}
                                    hasMore={hasMoreVersions}
                                    onShowMore={() => setVersionLimit(l => l + VERSION_PAGE_SIZE)}
                                />
                            ) : (
                                <ChipSection
                                    key={section.key}
                                    section={section}
                                    selected={filters[section.key]}
                                    onToggle={item =>
                                        onChange({...filters, [section.key]: toggle(filters[section.key], item)})
                                    }
                                />
                            )
                        )}
                    </div>
                </DrawerBody>

                <DrawerFooter>
                    <div className="flex items-center justify-between w-full">
                        <button
                            onClick={() => onChange(defaultFilterState)}
                            className="cursor-pointer"
                            style={{
                                background: "transparent",
                                border: "none",
                                color: "var(--ink-3)",
                                fontSize: 12,
                                fontWeight: 600,
                            }}
                        >
                            Clear all
                        </button>
                        <button
                            onClick={onClose}
                            className="cursor-pointer"
                            style={{
                                background: "var(--mc-green)",
                                color: "#072010",
                                border: "none",
                                borderRadius: 8,
                                padding: "8px 20px",
                                fontSize: 12,
                                fontWeight: 700,
                            }}
                        >
                            Show results
                        </button>
                    </div>
                </DrawerFooter>
            </DrawerContent>
        </Drawer>
    );
}

function SectionLabel({text}: {text: string}) {
    return (
        <div
            style={{
                fontSize: 10,
                fontWeight: 700,
                color: "var(--ink-3)",
                fontFamily: "var(--mono)",
                letterSpacing: 1,
                textTransform: "uppercase",
                marginBottom: 8,
            }}
        >
            {text}
        </div>
    );
}

function ChipSection({
    section,
    selected,
    onToggle,
}: {
    section: FilterSectionDef;
    selected: string[];
    onToggle: (item: string) => void;
}) {
    return (
        <div>
            <SectionLabel text={section.label}/>
            <div className="flex gap-2 flex-wrap">
                {section.items.map(item => (
                    <FilterChip
                        key={item}
                        label={item}
                        active={selected.includes(item)}
                        onClick={() => onToggle(item)}
                    />
                ))}
            </div>
        </div>
    );
}

function VersionSection({
    section,
    selected,
    onToggle,
    versions,
    versionSearch,
    onSearchChange,
    showSnapshots,
    onToggleSnapshots,
    hasMore,
    onShowMore,
}: {
    section: FilterSectionDef;
    selected: string[];
    onToggle: (item: string) => void;
    versions: string[];
    versionSearch: string;
    onSearchChange: (v: string) => void;
    showSnapshots: boolean;
    onToggleSnapshots: () => void;
    hasMore: boolean;
    onShowMore: () => void;
}) {
    return (
        <div>
            <SectionLabel text={section.label}/>
            <div className="flex items-center gap-2 mb-2">
                <Input
                    placeholder="Search versions..."
                    value={versionSearch}
                    onValueChange={onSearchChange}
                    size="sm"
                    variant="bordered"
                    startContent={<I.search size={12} style={{color: "var(--ink-3)"}}/>}
                    classNames={{
                        base: "flex-1",
                        inputWrapper: "border-line bg-[rgba(0,0,0,0.3)] h-8",
                        input: "text-xs",
                    }}
                />
                <button
                    onClick={onToggleSnapshots}
                    className="cursor-pointer flex-shrink-0"
                    style={{
                        padding: "5px 10px",
                        fontSize: 10,
                        fontWeight: 600,
                        borderRadius: 6,
                        border: showSnapshots
                            ? "1px solid rgba(34,255,132,0.4)"
                            : "1px solid var(--line)",
                        background: showSnapshots
                            ? "rgba(34,255,132,0.1)"
                            : "transparent",
                        color: showSnapshots ? "var(--mc-green)" : "var(--ink-3)",
                        transition: "all 0.12s",
                    }}
                >
                    Snapshots
                </button>
            </div>
            <div className="flex gap-1.5 flex-wrap">
                {versions.map(v => (
                    <FilterChip
                        key={v}
                        label={v}
                        active={selected.includes(v)}
                        onClick={() => onToggle(v)}
                    />
                ))}
            </div>
            {hasMore && (
                <button
                    onClick={onShowMore}
                    className="cursor-pointer mt-2"
                    style={{
                        background: "transparent",
                        border: "none",
                        color: "var(--mc-green)",
                        fontSize: 11,
                        fontWeight: 600,
                    }}
                >
                    Show more...
                </button>
            )}
        </div>
    );
}

function FilterChip({
    label,
    active,
    onClick,
}: {
    label: string;
    active: boolean;
    onClick: () => void;
}) {
    return (
        <button
            onClick={onClick}
            className="cursor-pointer"
            style={{
                padding: "5px 11px",
                fontSize: 11,
                fontWeight: 600,
                borderRadius: 999,
                border: active
                    ? "1px solid rgba(34,255,132,0.5)"
                    : "1px solid var(--line)",
                background: active
                    ? "rgba(34,255,132,0.14)"
                    : "rgba(255,255,255,0.04)",
                color: active ? "var(--mc-green)" : "var(--ink-2)",
                transition: "all 0.12s",
            }}
        >
            {label}
        </button>
    );
}
