import {I} from "../shell/icons";

const CATEGORIES = ["Adventure", "Magic", "Tech", "Exploration", "Survival", "Performance", "Quests", "Multiplayer"];
const LOADERS = ["Fabric", "Forge", "NeoForge", "Quilt", "Vanilla"];
const VERSIONS = ["1.21.4", "1.21.1", "1.20.4", "1.20.1", "1.19.4", "1.18.2", "1.16.5", "1.12.2"];
const ENVIRONMENTS = ["Client", "Server", "Client & Server"];

export interface FilterState {
    categories: string[];
    loaders: string[];
    versions: string[];
    environment: string[];
}

interface FilterPopoverProps {
    filters: FilterState;
    onChange: (f: FilterState) => void;
    onClose: () => void;
}

export default function FilterPopover({filters, onChange, onClose}: FilterPopoverProps) {
    const toggle = (arr: string[], item: string): string[] =>
        arr.includes(item) ? arr.filter(x => x !== item) : [...arr, item];

    const activeCount =
        filters.categories.length + filters.loaders.length +
        filters.versions.length + filters.environment.length;

    return (
        <div
            style={{
                position: "absolute",
                top: "calc(100% + 10px)",
                right: 0,
                width: 360,
                maxHeight: 520,
                display: "flex",
                flexDirection: "column",
                background: "rgba(14,16,18,0.96)",
                backdropFilter: "blur(24px)",
                border: "1px solid var(--line-strong)",
                borderRadius: 14,
                boxShadow: "0 24px 48px -12px rgba(0,0,0,0.6), 0 0 0 1px rgba(34,255,132,0.1)",
                zIndex: 50,
                overflow: "hidden",
            }}
        >
            {/* Header */}
            <div className="flex items-center justify-between" style={{padding: "14px 16px 10px"}}>
                <div className="flex items-center gap-2">
                    <I.filter size={14} style={{color: "var(--mc-green)"}}/>
                    <span style={{fontSize: 13, fontWeight: 700}}>Filters</span>
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
                <button
                    onClick={onClose}
                    className="cursor-pointer"
                    style={{
                        background: "transparent",
                        border: "none",
                        color: "var(--ink-3)",
                        padding: 4,
                    }}
                >
                    <I.x size={16}/>
                </button>
            </div>

            {/* Body */}
            <div style={{flex: 1, overflowY: "auto", padding: 16}} className="flex flex-col gap-4">
                <FilterSection
                    label="Category"
                    items={CATEGORIES}
                    selected={filters.categories}
                    onToggle={item => onChange({...filters, categories: toggle(filters.categories, item)})}
                />
                <FilterSection
                    label="Loader"
                    items={LOADERS}
                    selected={filters.loaders}
                    onToggle={item => onChange({...filters, loaders: toggle(filters.loaders, item)})}
                />
                <FilterSection
                    label="Minecraft Version"
                    items={VERSIONS}
                    selected={filters.versions}
                    onToggle={item => onChange({...filters, versions: toggle(filters.versions, item)})}
                />
                <FilterSection
                    label="Environment"
                    items={ENVIRONMENTS}
                    selected={filters.environment}
                    onToggle={item => onChange({...filters, environment: toggle(filters.environment, item)})}
                />
            </div>

            {/* Footer */}
            <div className="flex items-center justify-between" style={{padding: 12, borderTop: "1px solid var(--line)"}}>
                <button
                    onClick={() => onChange({categories: [], loaders: [], versions: [], environment: []})}
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
                        padding: "8px 16px",
                        fontSize: 12,
                        fontWeight: 700,
                    }}
                >
                    Apply
                </button>
            </div>
        </div>
    );
}

function FilterSection({label, items, selected, onToggle}: {
    label: string;
    items: string[];
    selected: string[];
    onToggle: (item: string) => void;
}) {
    return (
        <div>
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
                {label}
            </div>
            <div className="flex gap-2 flex-wrap">
                {items.map(item => {
                    const active = selected.includes(item);
                    return (
                        <button
                            key={item}
                            onClick={() => onToggle(item)}
                            className="cursor-pointer"
                            style={{
                                padding: "6px 12px",
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
                            {item}
                        </button>
                    );
                })}
            </div>
        </div>
    );
}
