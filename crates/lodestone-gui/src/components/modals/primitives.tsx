import React from "react";
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

// Styled text input row — dark background + subtle border. Optional leading icon.
export function InputRow({
                             value,
                             placeholder,
                             icon,
                             accent,
                             mono = false,
                         }: {
    value?: string;
    placeholder?: string;
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
                defaultValue={value}
                placeholder={placeholder}
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
// No actual dropdown yet; the design treats these as triggers.
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

// Minecraft version picker: version (mono) + "LATEST RELEASE" chip + chevron.
export function VersionPicker({version = "1.21.4"}: {version?: string}) {
    return (
        <div className="px-3 py-2.5 rounded-[9px] border border-line bg-[rgba(0,0,0,0.3)] flex items-center justify-between cursor-pointer">
            <div className="flex items-center gap-2">
                <div className="font-mono text-[0.8125rem] text-ink-0">{version}</div>
                <div
                    className="text-[0.625rem] px-1.5 py-0.5 rounded text-mc-green font-semibold"
                    style={{background: "color-mix(in oklab, var(--mc-green) 18%, transparent)"}}
                >
                    LATEST RELEASE
                </div>
            </div>
            <I.chevDown size={14} className="text-ink-3"/>
        </div>
    );
}

// Footer button — primary variant takes the modal's accent color and glows.
export function FooterBtn({
                              children,
                              primary,
                              accent = "var(--mc-green)",
                              onClick,
                          }: {
    children: React.ReactNode;
    primary?: boolean;
    accent?: string;
    onClick?: () => void;
}) {
    if (primary) {
        return (
            <button
                type="button"
                onClick={onClick}
                className="px-[18px] py-2.5 rounded-[9px] text-[0.8125rem] font-semibold cursor-pointer"
                style={{
                    background: accent,
                    color: "#072010",
                    border: `1px solid ${accent}`,
                    boxShadow: `0 0 20px color-mix(in oklab, ${accent} 45%, transparent)`,
                }}
            >
                {children}
            </button>
        );
    }
    return (
        <button
            type="button"
            onClick={onClick}
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
