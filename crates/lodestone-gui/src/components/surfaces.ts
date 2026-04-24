import React from "react";

// Shared "glassy card" surface gradient — layered linear-gradient w/ rgba stops
// doesn't cleanly reduce to a single Tailwind utility, so it stays inline.
// Reused by Library, Instance Detail, Modals, Worlds, and Settings cards.
export const cardSurfaceStyle: React.CSSProperties = {
    background:
        "linear-gradient(180deg, rgba(255,255,255,0.035) 0%, rgba(255,255,255,0.015) 100%)",
};

// Design's .card:hover effect: subtle lift, green-tinted border, drop shadow
// with a faint green rim. Applied to all pressable instance/action cards.
export const cardHoverClass =
    "transition-all duration-200 hover:-translate-y-0.5 hover:border-[rgba(34,255,132,0.25)] hover:shadow-[0_20px_48px_-24px_rgba(0,0,0,0.6),0_0_0_1px_rgba(34,255,132,0.15)]";
