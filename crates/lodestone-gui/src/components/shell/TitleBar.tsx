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
            className="flex items-center gap-4 px-7 py-[18px] border-b border-line relative z-10 bg-[rgba(8,9,10,0.7)] backdrop-blur-xl"
        >
            <div>
                <div className="text-xl font-bold tracking-tight">{title}</div>
                {subtitle && <div className="text-xs text-ink-3 mt-0.5">{subtitle}</div>}
            </div>
            <div className="flex-1"/>
            {children}
        </div>
    );
}
