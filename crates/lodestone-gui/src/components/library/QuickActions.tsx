import React from "react";
import {Card} from "@heroui/react";
import {I} from "../shell/icons";
import {cardHoverClass, cardSurfaceStyle} from "./instances";

export type QuickActionKey = "new" | "import" | "server" | "coop";

type Tint = "green" | "cyan" | "amber" | "violet";

// Lookup table used to tint quick-action tiles without assembling color-mix()
// strings inline for every render. Keyed by the design's semantic color token.
const quickActionTints: Record<Tint, {icon: string; bg: string; border: string}> = {
    green: {
        icon: "text-mc-green",
        bg: "bg-[color-mix(in_oklab,var(--mc-green)_18%,transparent)]",
        border: "border-[color-mix(in_oklab,var(--mc-green)_30%,transparent)]",
    },
    cyan: {
        icon: "text-accent-cyan",
        bg: "bg-[color-mix(in_oklab,var(--cyan)_18%,transparent)]",
        border: "border-[color-mix(in_oklab,var(--cyan)_30%,transparent)]",
    },
    amber: {
        icon: "text-accent-amber",
        bg: "bg-[color-mix(in_oklab,var(--amber)_18%,transparent)]",
        border: "border-[color-mix(in_oklab,var(--amber)_30%,transparent)]",
    },
    violet: {
        icon: "text-[#b689ff]",
        bg: "bg-[color-mix(in_oklab,var(--violet)_18%,transparent)]",
        border: "border-[color-mix(in_oklab,var(--violet)_30%,transparent)]",
    },
};

type QuickAction = {
    icon: (p: {size?: number}) => React.ReactElement;
    label: string;
    sub: string;
    tint: Tint;
    key: QuickActionKey;
};

const quickActions: QuickAction[] = [
    {icon: I.plus, label: "New Instance", sub: "From modpack, CurseForge, or scratch", tint: "green", key: "new"},
    {icon: I.download, label: "Import", sub: ".zip, .mrpack, CurseForge", tint: "cyan", key: "import"},
    {icon: I.server, label: "Join Server", sub: "Realms · hosted · friends", tint: "amber", key: "server"},
    {icon: I.users, label: "Co-op Sync", sub: "3 friends online now", tint: "violet", key: "coop"},
];

type QuickActionsProps = {
    onActionPress: (key: QuickActionKey) => void;
};

// Four-tile grid of library quick actions under the hero banner. Each card's
// onPress fires the parent's handler with the action's discriminator.
export default function QuickActions({onActionPress}: QuickActionsProps) {
    return (
        <div className="grid grid-cols-4 gap-3 mb-8">
            {quickActions.map((a, i) => {
                const IconC = a.icon;
                const tint = quickActionTints[a.tint];
                return (
                    <Card
                        key={i}
                        isPressable
                        onPress={() => onActionPress(a.key)}
                        className={`flex-row items-center gap-3.5 px-[18px] py-4 border border-line ${cardHoverClass}`}
                        style={cardSurfaceStyle}
                    >
                        <div
                            className={`flex-shrink-0 w-10 h-10 rounded-[10px] flex items-center justify-center border ${tint.bg} ${tint.border} ${tint.icon}`}
                        >
                            <IconC size={18}/>
                        </div>
                        <div className="min-w-0 text-left">
                            <div className="text-[0.8125rem] font-semibold">{a.label}</div>
                            <div className="text-[0.6875rem] text-ink-3 mt-0.5">{a.sub}</div>
                        </div>
                    </Card>
                );
            })}
        </div>
    );
}
