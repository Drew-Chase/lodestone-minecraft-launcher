import React from "react";

type TitleBarProps = {
    title: string;
    subtitle?: string;
    children?: React.ReactNode;
};

// Page header strip (per-screen title + right-side actions).
export default function TitleBar({title, subtitle, children}: TitleBarProps) {
    return (
        <div
            className="flex items-center border-b border-line relative z-10"
            style={{
                gap: 16,
                padding: "18px 28px",
                background: "rgba(8,9,10,0.7)",
                backdropFilter: "blur(20px)",
            }}
        >
            <div>
                <div style={{fontSize: 20, fontWeight: 700, letterSpacing: -0.3}}>{title}</div>
                {subtitle && (
                    <div style={{fontSize: 12, color: "var(--ink-3)", marginTop: 2}}>{subtitle}</div>
                )}
            </div>
            <div style={{flex: 1}}/>
            {children}
        </div>
    );
}
