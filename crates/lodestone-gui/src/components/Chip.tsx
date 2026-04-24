import React from "react";

type ChipVariant = "neutral" | "green" | "violet" | "amber" | "cyan" | "pink";

type ChipProps = {
    variant?: ChipVariant;
    children: React.ReactNode;
    className?: string;
    style?: React.CSSProperties;
};

const variantStyles: Record<ChipVariant, {background: string; color: string}> = {
    neutral: {background: "rgba(255,255,255,0.06)", color: "var(--ink-2)"},
    green: {background: "rgba(34,255,132,0.12)", color: "var(--mc-green)"},
    violet: {background: "rgba(151,71,255,0.14)", color: "#b689ff"},
    amber: {background: "rgba(255,181,69,0.14)", color: "var(--amber)"},
    cyan: {background: "rgba(71,217,255,0.14)", color: "var(--cyan)"},
    pink: {background: "rgba(255,94,200,0.14)", color: "var(--pink)"},
};

// Mono uppercase pill used heavily in the design (e.g. "NOW PLAYING", "FABRIC 0.14.21").
export default function Chip({variant = "neutral", children, className, style}: ChipProps) {
    const v = variantStyles[variant];
    return (
        <span
            className={`font-mono uppercase${className ? ` ${className}` : ""}`}
            style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 6,
                fontSize: 11,
                fontWeight: 600,
                padding: "4px 8px",
                borderRadius: 999,
                letterSpacing: "0.02em",
                background: v.background,
                color: v.color,
                ...style,
            }}
        >
            {children}
        </span>
    );
}
