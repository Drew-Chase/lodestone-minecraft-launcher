import React from "react";

type ChipVariant = "neutral" | "green" | "violet" | "amber" | "cyan" | "pink";

type ChipProps = {
    variant?: ChipVariant;
    children: React.ReactNode;
    className?: string;
    style?: React.CSSProperties;
};

// Variant-specific background + text color pairs — kept as Tailwind arbitrary classes
// so the rgba(a,b,c, .12) tints stay in one place.
const variantClasses: Record<ChipVariant, string> = {
    neutral: "bg-[rgba(255,255,255,0.06)] text-ink-2",
    green: "bg-[rgba(34,255,132,0.12)] text-mc-green",
    violet: "bg-[rgba(151,71,255,0.14)] text-[#b689ff]",
    amber: "bg-[rgba(255,181,69,0.14)] text-accent-amber",
    cyan: "bg-[rgba(71,217,255,0.14)] text-accent-cyan",
    pink: "bg-[rgba(255,94,200,0.14)] text-accent-pink",
};

// Mono uppercase pill used heavily in the design (e.g. "NOW PLAYING", "FABRIC 0.14.21").
export default function Chip({variant = "neutral", children, className, style}: ChipProps) {
    return (
        <span
            className={[
                "inline-flex items-center gap-1.5 font-mono uppercase font-semibold rounded-full text-[0.6875rem] px-2 py-1 tracking-[0.02em]",
                variantClasses[variant],
                className ?? "",
            ]
                .filter(Boolean)
                .join(" ")}
            style={style}
        >
      {children}
    </span>
    );
}
