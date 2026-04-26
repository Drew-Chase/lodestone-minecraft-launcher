import React, {useRef, useState} from "react";
import {I} from "../shell/icons";

// Small mono-uppercase label used above form rows in every modal.
export function Label({children, className}: {children: React.ReactNode; className?: string}) {
    return (
        <div
            className={`font-mono uppercase text-[0.625rem] font-semibold tracking-[0.1em] text-ink-3 mb-1.5${
                className ? ` ${className}` : ""
            }`}
        >
            {children}
        </div>
    );
}

// Styled text input row — dark background + subtle border.
export function InputRow({
                             value,
                             placeholder,
                             onChange,
                             icon,
                             accent,
                             mono = false,
                         }: {
    value?: string;
    placeholder?: string;
    onChange?: (value: string) => void;
    icon?: (p: {size?: number; style?: React.CSSProperties; className?: string}) => React.ReactElement;
    accent?: string;
    mono?: boolean;
}) {
    const IC = icon;
    return (
        <div className="relative">
            {IC && (
                <span
                    className="absolute left-3 top-1/2 -translate-y-1/2"
                    style={{color: accent ?? "var(--ink-3)"}}
                >
          <IC size={14}/>
        </span>
            )}
            <input
                value={value}
                placeholder={placeholder}
                onChange={onChange ? (e) => onChange(e.target.value) : undefined}
                className={[
                    "w-full rounded-[9px] border text-ink-0 text-[0.8125rem] outline-none box-border",
                    "bg-[rgba(0,0,0,0.3)]",
                    IC ? "pl-[34px] pr-3 py-2.5" : "px-3 py-2.5",
                    mono ? "font-mono" : "font-sans",
                ].join(" ")}
                style={{
                    borderColor: accent
                        ? `color-mix(in oklab, ${accent} 22%, var(--line))`
                        : "var(--line)",
                }}
            />
        </div>
    );
}

// Static "select" visual — dark rectangle with value + optional hint + chevron.
export function SelectRow({value, hint}: {value: string; hint?: string}) {
    return (
        <div className="px-3 py-2.5 rounded-[9px] border border-line bg-[rgba(0,0,0,0.3)] flex items-center justify-between cursor-pointer">
            <div>
                <div className="text-[0.8125rem] text-ink-0">{value}</div>
                {hint && <div className="text-[0.625rem] text-ink-3 mt-px">{hint}</div>}
            </div>
            <I.chevDown size={14} className="text-ink-3"/>
        </div>
    );
}

// Searchable dropdown for Minecraft/loader version selection.
export function VersionDropdown({
                                    versions,
                                    selected,
                                    onChange,
                                    latestVersion,
                                    placeholder = "Select version",
                                    loading = false,
                                }: {
    versions: string[];
    selected: string;
    onChange: (version: string) => void;
    latestVersion?: string;
    placeholder?: string;
    loading?: boolean;
}) {
    const [open, setOpen] = useState(false);
    const [search, setSearch] = useState("");
    const containerRef = useRef<HTMLDivElement>(null);

    const filtered = search
        ? versions.filter((v) => v.toLowerCase().includes(search.toLowerCase()))
        : versions;

    const isLatest = selected && selected === latestVersion;

    return (
        <div className="relative" ref={containerRef}>
            <div
                className="px-3 py-2.5 rounded-[9px] border border-line bg-[rgba(0,0,0,0.3)] flex items-center justify-between cursor-pointer"
                onClick={() => !loading && setOpen(!open)}
            >
                <div className="flex items-center gap-2">
                    {loading ? (
                        <div className="flex items-center gap-2 text-ink-3">
                            <div className="w-3 h-3 border border-ink-3 border-t-transparent rounded-full animate-spin"/>
                            <span className="text-[0.8125rem]">Loading…</span>
                        </div>
                    ) : selected ? (
                        <>
                            <div className="font-mono text-[0.8125rem] text-ink-0">{selected}</div>
                            {isLatest && (
                                <div
                                    className="text-[0.625rem] px-1.5 py-0.5 rounded text-mc-green font-semibold"
                                    style={{background: "color-mix(in oklab, var(--mc-green) 18%, transparent)"}}
                                >
                                    LATEST
                                </div>
                            )}
                        </>
                    ) : (
                        <span className="text-[0.8125rem] text-ink-3">{placeholder}</span>
                    )}
                </div>
                <I.chevDown size={14} className="text-ink-3"/>
            </div>
            {open && (
                <div
                    className="absolute z-50 mt-1 w-full max-h-[240px] rounded-[9px] border border-line bg-bg-1 overflow-hidden flex flex-col"
                    style={{boxShadow: "0 12px 40px rgba(0,0,0,0.5)"}}
                >
                    <div className="px-2 pt-2 pb-1">
                        <input
                            autoFocus
                            placeholder="Search…"
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            className="w-full px-2.5 py-1.5 rounded-md border border-line bg-[rgba(0,0,0,0.3)] text-ink-0 text-[0.75rem] font-mono outline-none"
                        />
                    </div>
                    <div className="overflow-y-auto flex-1">
                        {filtered.length === 0 && (
                            <div className="px-3 py-3 text-[0.75rem] text-ink-3 text-center">No versions found</div>
                        )}
                        {filtered.map((v) => (
                            <button
                                key={v}
                                type="button"
                                className={[
                                    "w-full px-3 py-1.5 text-left text-[0.75rem] font-mono cursor-pointer bg-transparent border-none transition-colors font-sans",
                                    v === selected
                                        ? "text-mc-green bg-[rgba(34,255,132,0.08)]"
                                        : "text-ink-1 hover:bg-[rgba(255,255,255,0.04)]",
                                ].join(" ")}
                                onClick={() => {
                                    onChange(v);
                                    setOpen(false);
                                    setSearch("");
                                }}
                            >
                                <span className="font-mono">{v}</span>
                                {v === latestVersion && (
                                    <span className="ml-2 text-[0.5625rem] text-mc-green font-semibold">LATEST</span>
                                )}
                            </button>
                        ))}
                    </div>
                </div>
            )}
            {/* Close dropdown on outside click */}
            {open && (
                <div
                    className="fixed inset-0 z-40"
                    onClick={() => {
                        setOpen(false);
                        setSearch("");
                    }}
                />
            )}
        </div>
    );
}

// Footer button — primary variant takes the modal's accent color and glows.
export function FooterBtn({
                              children,
                              primary,
                              accent = "var(--mc-green)",
                              onClick,
                              disabled,
                              loading,
                          }: {
    children: React.ReactNode;
    primary?: boolean;
    accent?: string;
    onClick?: () => void;
    disabled?: boolean;
    loading?: boolean;
}) {
    if (primary) {
        return (
            <button
                type="button"
                onClick={onClick}
                disabled={disabled || loading}
                className={[
                    "px-[18px] py-2.5 rounded-[9px] text-[0.8125rem] font-semibold cursor-pointer flex items-center gap-2",
                    disabled ? "opacity-50 cursor-not-allowed" : "",
                ].join(" ")}
                style={{
                    background: accent,
                    color: "#072010",
                    border: `1px solid ${accent}`,
                    boxShadow: disabled ? "none" : `0 0 20px color-mix(in oklab, ${accent} 45%, transparent)`,
                }}
            >
                {loading && <div className="w-3.5 h-3.5 border-2 border-current border-t-transparent rounded-full animate-spin"/>}
                {children}
            </button>
        );
    }
    return (
        <button
            type="button"
            onClick={onClick}
            disabled={disabled}
            className="px-[18px] py-2.5 rounded-[9px] text-[0.8125rem] font-semibold cursor-pointer bg-transparent text-ink-1 border border-line hover:bg-[rgba(255,255,255,0.04)]"
        >
            {children}
        </button>
    );
}

// 4-bar signal indicator, colored by ping bucket. Used in Join Server modal.
export function PingBars({ping}: {ping: number}) {
    const bars = ping < 30 ? 4 : ping < 60 ? 3 : ping < 120 ? 2 : 1;
    const color =
        ping < 30 ? "var(--mc-green)" : ping < 60 ? "var(--amber)" : "#ff6b6b";
    const heights = [5, 7, 9, 11];
    return (
        <div className="flex items-end gap-px">
            {heights.map((h, i) => (
                <div
                    key={i}
                    style={{
                        width: 2,
                        height: h,
                        background: i < bars ? color : "var(--line)",
                        borderRadius: 0.5,
                    }}
                />
            ))}
        </div>
    );
}
