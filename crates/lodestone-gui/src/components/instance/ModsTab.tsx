import {Button, Input} from "@heroui/react";
import Chip from "../Chip";
import {Switch} from "../Switch";
import {I} from "../shell/icons";
import type {ChipColor} from "../library/instances";
import {cardSurfaceStyle} from "../library/instances";

type Mod = {
    name: string;
    author: string;
    ver: string;
    cat: string;
    color: ChipColor;
    on: boolean;
};

const mods: Mod[] = [
    {name: "Sodium", author: "JellySquid", ver: "0.5.8", cat: "Performance", color: "cyan", on: true},
    {name: "Iris Shaders", author: "coderbot", ver: "1.7.1", cat: "Graphics", color: "violet", on: true},
    {name: "Lithium", author: "CaffeineMC", ver: "0.12.1", cat: "Performance", color: "cyan", on: true},
    {name: "Fabric API", author: "FabricMC", ver: "0.92.0", cat: "Library", color: "amber", on: true},
    {name: "Xaero's Minimap", author: "xaero96", ver: "24.1.0", cat: "HUD", color: "green", on: true},
    {name: "JEI", author: "mezz", ver: "15.2.0", cat: "Utility", color: "amber", on: true},
    {name: "Mod Menu", author: "Prospector", ver: "7.2.2", cat: "Utility", color: "amber", on: false},
    {name: "Continuity", author: "PepperCode1", ver: "3.0.0", cat: "Graphics", color: "violet", on: true},
];

// Icon-tile accent colors per mod category — mirrors the chip variant palette.
const iconTintByColor: Record<ChipColor, {bg: string; border: string; text: string}> = {
    green: {
        bg: "bg-[color-mix(in_oklab,var(--mc-green)_14%,transparent)]",
        border: "border-[color-mix(in_oklab,var(--mc-green)_28%,transparent)]",
        text: "text-mc-green",
    },
    violet: {
        bg: "bg-[color-mix(in_oklab,var(--violet)_14%,transparent)]",
        border: "border-[color-mix(in_oklab,var(--violet)_28%,transparent)]",
        text: "text-accent-violet",
    },
    amber: {
        bg: "bg-[color-mix(in_oklab,var(--amber)_14%,transparent)]",
        border: "border-[color-mix(in_oklab,var(--amber)_28%,transparent)]",
        text: "text-accent-amber",
    },
    cyan: {
        bg: "bg-[color-mix(in_oklab,var(--cyan)_14%,transparent)]",
        border: "border-[color-mix(in_oklab,var(--cyan)_28%,transparent)]",
        text: "text-accent-cyan",
    },
    pink: {
        bg: "bg-[color-mix(in_oklab,var(--pink)_14%,transparent)]",
        border: "border-[color-mix(in_oklab,var(--pink)_28%,transparent)]",
        text: "text-accent-pink",
    },
};

// Mods tab — filter row pinned at top + scrollable list card beneath it. Each
// mod shows a category-tinted icon tile, name with category chip, author/version
// mono line, toggle, and overflow. The outer flex-col wrapper lets the header
// stay put while only the card below scrolls.
export default function ModsTab() {
    return (
        <div className="flex-1 flex flex-col min-h-0 px-7 pt-5 pb-5">
            {/* Filter row — fixed, never scrolls */}
            <div className="flex-shrink-0 flex gap-2.5 mb-3.5">
                <Input
                    placeholder="Filter mods…"
                    size="sm"
                    classNames={{
                        base: "flex-1",
                        inputWrapper: "bg-[rgba(255,255,255,0.04)] border border-line",
                    }}
                    startContent={<I.search size={14}/>}
                />
                <Button
                    variant="bordered"
                    size="sm"
                    startContent={<I.filter size={13}/>}
                >
                    Category
                </Button>
                <Button
                    color="success"
                    size="sm"
                    className="font-bold"
                    startContent={<I.plus size={12}/>}
                >
                    Add Mod
                </Button>
            </div>

            {/* Mods list — scrollable region. Plain div rather than HeroUI Card
                so flex-1 + overflow-y-auto aren't fighting Card's own flex layout. */}
            <div
                className="flex-1 min-h-0 overflow-y-auto rounded-md border border-line"
                style={cardSurfaceStyle}
            >
                {mods.map((m, i) => {
                    const tint = iconTintByColor[m.color];
                    const isLast = i === mods.length - 1;
                    return (
                        <div
                            key={i}
                            className={`flex items-center gap-3.5 px-4 py-3 transition-colors hover:bg-[rgba(255,255,255,0.02)] ${
                                isLast ? "" : "border-b border-line"
                            }`}
                        >
                            <div
                                className={`flex-shrink-0 w-8 h-8 rounded-lg flex items-center justify-center border ${tint.bg} ${tint.border} ${tint.text}`}
                            >
                                <I.box size={14}/>
                            </div>
                            <div className="min-w-0 flex-1">
                                <div className="text-[0.8125rem] font-semibold flex items-center gap-2">
                                    {m.name}
                                    <Chip variant={m.color} className="text-[0.5625rem] px-1.5 py-px">
                                        {m.cat}
                                    </Chip>
                                </div>
                                <div className="text-[0.6875rem] text-ink-3 mt-0.5 font-mono">
                                    by {m.author} · v{m.ver}
                                </div>
                            </div>
                            <Switch
                                isSelected={m.on}
                                size="sm"
                                color="success"
                                aria-label={`Toggle ${m.name}`}
                            />
                            <Button
                                isIconOnly
                                variant="bordered"
                                size="sm"
                                aria-label="More"
                                className="w-7 h-7 min-w-0"
                            >
                                <I.more size={13}/>
                            </Button>
                        </div>
                    );
                })}
            </div>
        </div>
    );
}
