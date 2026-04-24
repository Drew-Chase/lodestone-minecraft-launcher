import React, {useState} from "react";
import {Button, Card, Input} from "@heroui/react";
import Scene, {Biome} from "../components/shell/Scene";
import Particles from "../components/shell/Particles";
import TitleBar from "../components/shell/TitleBar";
import Chip from "../components/Chip";
import {I} from "../components/shell/icons";

type ChipColor = "green" | "violet" | "amber" | "cyan" | "pink";
type Loader = "Fabric" | "Vanilla" | "Forge" | "NeoForge" | "Quilt";

type Instance = {
    name: string;
    version: string;
    loader: Loader;
    mc: string;
    biome: Biome;
    seed: number;
    playtime: string;
    lastPlayed: string;
    mods: number;
    playing?: boolean;
    color: ChipColor;
};

const instances: Instance[] = [
    {
        name: "Aether Legacy",
        version: "1.20.1 · Fabric",
        loader: "Fabric",
        mc: "1.20.1",
        biome: "end",
        seed: 3,
        playtime: "24h",
        lastPlayed: "12m ago",
        mods: 87,
        playing: true,
        color: "violet",
    },
    {
        name: "Vanilla Survival",
        version: "1.20.4 · Vanilla",
        loader: "Vanilla",
        mc: "1.20.4",
        biome: "forest",
        seed: 1,
        playtime: "156h",
        lastPlayed: "2d ago",
        mods: 0,
        color: "green",
    },
    {
        name: "Create: Above & Beyond",
        version: "1.16.5 · Forge",
        loader: "Forge",
        mc: "1.16.5",
        biome: "desert",
        seed: 5,
        playtime: "42h",
        lastPlayed: "5d ago",
        mods: 214,
        color: "amber",
    },
    {
        name: "RLCraft Extreme",
        version: "1.12.2 · Forge",
        loader: "Forge",
        mc: "1.12.2",
        biome: "nether",
        seed: 2,
        playtime: "8h",
        lastPlayed: "3w ago",
        mods: 171,
        color: "pink",
    },
    {
        name: "SkyFactory 4",
        version: "1.12.2 · Forge",
        loader: "Forge",
        mc: "1.12.2",
        biome: "ocean",
        seed: 8,
        playtime: "67h",
        lastPlayed: "1w ago",
        mods: 198,
        color: "cyan",
    },
    {
        name: "Cherry Grove Peaceful",
        version: "1.20.4 · Vanilla",
        loader: "Vanilla",
        mc: "1.20.4",
        biome: "cherry",
        seed: 4,
        playtime: "3h",
        lastPlayed: "1mo ago",
        mods: 0,
        color: "pink",
    },
];

const tabs = ["Recent", "Pinned", "Modded", "Vanilla"] as const;
type Tab = (typeof tabs)[number];

type ViewMode = "grid" | "compact" | "table";

// Lookup table used to tint quick-action tiles without assembling color-mix()
// strings inline for every render. Keyed by the design's semantic color token.
const quickActionTints: Record<
    "green" | "cyan" | "amber" | "violet",
    {icon: string; bg: string; border: string}
> = {
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
    tint: keyof typeof quickActionTints;
};

const quickActions: QuickAction[] = [
    {icon: I.plus, label: "New Instance", sub: "From modpack, CurseForge, or scratch", tint: "green"},
    {icon: I.download, label: "Import", sub: ".zip, .mrpack, CurseForge", tint: "cyan"},
    {icon: I.server, label: "Join Server", sub: "Realms · hosted · friends", tint: "amber"},
    {icon: I.users, label: "Co-op Sync", sub: "3 friends online now", tint: "violet"},
];

// Shared "glassy card" surface gradient — layered linear-gradient w/ rgba stops
// doesn't cleanly reduce to a single Tailwind utility, so it stays inline.
const cardSurfaceStyle: React.CSSProperties = {
    background:
        "linear-gradient(180deg, rgba(255,255,255,0.035) 0%, rgba(255,255,255,0.015) 100%)",
};

export default function Home() {
    const [viewMode, setViewMode] = useState<ViewMode>("grid");
    const [activeTab, setActiveTab] = useState<Tab>("Recent");

    const featured = instances[0];

    return (
        <div className="flex flex-col flex-1 min-w-0 min-h-0 bg-bg-0">
            <TitleBar title="Library" subtitle="6 instances · 300h total playtime">
                <Input
                    placeholder="Search instances…"
                    size="sm"
                    classNames={{
                        base: "w-[240px]",
                        inputWrapper: "bg-[rgba(255,255,255,0.04)] border border-line",
                    }}
                    startContent={<I.search size={14}/>}
                />
                <Button isIconOnly variant="bordered" size="sm" aria-label="Refresh">
                    <I.refresh size={16}/>
                </Button>
                <Button isIconOnly variant="bordered" size="sm" aria-label="Notifications">
                    <I.bell size={16}/>
                </Button>
                <Button
                    color="success"
                    size="sm"
                    className="font-bold"
                    startContent={<I.plus size={14}/>}
                >
                    New Instance
                </Button>
            </TitleBar>

            <div className="flex-1 overflow-y-auto px-7 pt-6 pb-10">
                {/* Hero — currently playing */}
                <div
                    className="relative h-[260px] rounded-[20px] overflow-hidden mb-7 shadow-[0_20px_40px_-20px_rgba(0,0,0,0.8)]"
                >
                    <Scene biome={featured.biome} seed={featured.seed}/>
                    <Particles count={12}/>
                    <div
                        className="absolute inset-0 flex items-end p-8"
                        style={{
                            // Horizontal fade uses three rgba stops — keep inline.
                            background:
                                "linear-gradient(90deg, rgba(8,9,10,0.92) 0%, rgba(8,9,10,0.6) 45%, transparent 80%)",
                        }}
                    >
                        <div className="max-w-[500px]">
                            <div className="flex items-center gap-2 mb-3">
                                <Chip variant="green">
                                    <span className="pulse-dot" style={{width: 6, height: 6}}/> NOW PLAYING
                                </Chip>
                                <Chip variant="violet">FABRIC 0.14.21</Chip>
                                <Chip>MC 1.20.1</Chip>
                            </div>
                            <div className="text-[40px] font-extrabold -tracking-[1px] leading-none mb-2">
                                {featured.name}
                            </div>
                            <div className="text-[13px] text-ink-2 mb-5 leading-relaxed">
                                Climb the floating islands, battle Slider bosses, uncover a continent hanging in the
                                void.
                                <span className="text-mc-green ml-1.5">{featured.mods} mods</span>{" "}
                                · last played {featured.lastPlayed}
                            </div>
                            <div className="flex gap-2.5">
                                <Button
                                    color="success"
                                    size="lg"
                                    className="font-bold px-6"
                                    startContent={<I.play size={14}/>}
                                >
                                    Resume Session
                                </Button>
                                <Button
                                    isIconOnly
                                    variant="bordered"
                                    aria-label="Open Folder"
                                    className="w-[42px] h-[42px]"
                                >
                                    <I.folder size={16}/>
                                </Button>
                                <Button
                                    isIconOnly
                                    variant="bordered"
                                    aria-label="Configure"
                                    className="w-[42px] h-[42px]"
                                >
                                    <I.settings size={16}/>
                                </Button>
                                <Button
                                    isIconOnly
                                    variant="bordered"
                                    aria-label="More"
                                    className="w-[42px] h-[42px]"
                                >
                                    <I.more size={16}/>
                                </Button>
                            </div>
                        </div>
                    </div>
                    {/* Stats pill */}
                    <div className="font-mono absolute top-6 right-6 text-[11px] text-ink-2 px-2.5 py-1.5 rounded-md border border-[rgba(255,255,255,0.08)] bg-[rgba(0,0,0,0.55)] backdrop-blur-md flex items-center gap-1.5">
                        <I.cpu size={12}/> 2.1 GB / 6 GB · 58 FPS
                    </div>
                </div>

                {/* Quick actions */}
                <div className="grid grid-cols-4 gap-3 mb-8">
                    {quickActions.map((a, i) => {
                        const IconC = a.icon;
                        const tint = quickActionTints[a.tint];
                        return (
                            <Card
                                key={i}
                                isPressable
                                className="flex-row items-center gap-3.5 px-[18px] py-4 border border-line"
                                style={cardSurfaceStyle}
                            >
                                <div
                                    className={`flex-shrink-0 w-10 h-10 rounded-[10px] flex items-center justify-center border ${tint.bg} ${tint.border} ${tint.icon}`}
                                >
                                    <IconC size={18}/>
                                </div>
                                <div className="min-w-0 text-left">
                                    <div className="text-[13px] font-semibold">{a.label}</div>
                                    <div className="text-[11px] text-ink-3 mt-0.5">{a.sub}</div>
                                </div>
                            </Card>
                        );
                    })}
                </div>

                {/* Section header */}
                <div className="flex items-center mb-3.5 gap-3">
                    <div className="text-base font-bold tracking-tight">Your Instances</div>
                    <div className="flex-1"/>
                    <div className="flex gap-1.5">
                        {tabs.map((t) => (
                            <TabPill key={t} active={activeTab === t} onClick={() => setActiveTab(t)}>
                                {t}
                            </TabPill>
                        ))}
                    </div>
                    <ViewModeToggle mode={viewMode} onChange={setViewMode}/>
                </div>

                {/* Instance list */}
                {viewMode === "grid" && <InstanceGrid list={instances}/>}
                {viewMode === "compact" && <InstanceCompact list={instances}/>}
                {viewMode === "table" && <InstanceTable list={instances}/>}
            </div>
        </div>
    );
}

function TabPill({
                     active,
                     children,
                     onClick,
                 }: {
    active: boolean;
    children: React.ReactNode;
    onClick: () => void;
}) {
    return (
        <button
            onClick={onClick}
            className={[
                "font-sans px-3 py-1.5 text-xs font-medium rounded-md border-none cursor-pointer transition-colors",
                active
                    ? "text-bg-0 bg-mc-green shadow-[0_0_0_1px_rgba(34,255,132,0.6),0_8px_20px_-8px_rgba(34,255,132,0.35)]"
                    : "text-ink-2 bg-transparent",
            ].join(" ")}
        >
            {children}
        </button>
    );
}

function ViewModeToggle({
                            mode,
                            onChange,
                        }: {
    mode: ViewMode;
    onChange: (m: ViewMode) => void;
}) {
    const opts: {id: ViewMode; icon: React.FC<{size?: number}>; label: string}[] = [
        {id: "grid", icon: I.grid, label: "Grid"},
        {id: "compact", icon: I.list, label: "Compact"},
        {id: "table", icon: I.table, label: "Table"},
    ];
    return (
        <div className="flex p-[3px] gap-0.5 bg-[rgba(255,255,255,0.04)] border border-line rounded-[10px]">
            {opts.map((o) => {
                const IconC = o.icon;
                const active = mode === o.id;
                return (
                    <button
                        key={o.id}
                        onClick={() => onChange(o.id)}
                        title={`${o.label} view`}
                        className={[
                            "font-sans flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-semibold rounded-[7px] border-none cursor-pointer",
                            active
                                ? "bg-[color-mix(in_oklab,var(--mc-green)_18%,transparent)] text-mc-green shadow-[inset_0_0_0_1px_color-mix(in_oklab,var(--mc-green)_40%,transparent)]"
                                : "bg-transparent text-ink-2",
                        ].join(" ")}
                    >
                        <IconC size={13}/>
                        <span>{o.label}</span>
                    </button>
                );
            })}
        </div>
    );
}

function InstanceGrid({list}: {list: Instance[]}) {
    return (
        <div className="grid grid-cols-3 gap-4">
            {list.map((inst, i) => (
                <Card
                    key={i}
                    isPressable
                    className="cursor-pointer overflow-hidden p-0 border border-line w-full"
                    style={cardSurfaceStyle}
                >
                    <div className="relative h-[140px] w-full">
                        <Scene biome={inst.biome} seed={inst.seed}/>
                        {inst.playing && (
                            <div className="absolute top-2.5 left-2.5 z-[2]">
                                <Chip variant="green" className="text-[9px] px-1.5 py-0.5">
                                    <span className="pulse-dot" style={{width: 5, height: 5}}/>
                                    PLAYING
                                </Chip>
                            </div>
                        )}
                        <div className="absolute top-2.5 right-2.5 z-[2]">
                            <Button
                                isIconOnly
                                size="sm"
                                variant="flat"
                                aria-label="More"
                                className="w-7 h-7 min-w-0 bg-[rgba(0,0,0,0.5)] backdrop-blur-sm"
                            >
                                <I.more size={14}/>
                            </Button>
                        </div>
                        <div
                            className="absolute inset-0"
                            style={{
                                background:
                                    "linear-gradient(180deg, transparent 55%, rgba(0,0,0,0.7) 100%)",
                            }}
                        />
                        <div className="absolute bottom-2.5 left-3 right-3 flex items-end z-[2]">
                            <div className="min-w-0 flex-1">
                                <div className="text-sm font-bold mb-0.5">{inst.name}</div>
                                <div className="font-mono text-[10px] text-ink-2">{inst.version}</div>
                            </div>
                            <Button
                                color="success"
                                size="sm"
                                className="font-bold"
                                startContent={<I.play size={11}/>}
                            >
                                Play
                            </Button>
                        </div>
                    </div>
                    <div className="px-3.5 py-3 flex items-center gap-2.5 text-[11px] text-ink-3">
            <span className="flex items-center gap-1">
              <I.clock size={11}/> {inst.playtime}
            </span>
                        <span>·</span>
                        <span className="flex items-center gap-1">
              <I.box size={11}/> {inst.mods} mods
            </span>
                        <div className="flex-1"/>
                        <Chip variant={inst.color} className="text-[9px] px-1.5 py-0.5">
                            {inst.loader}
                        </Chip>
                    </div>
                </Card>
            ))}
        </div>
    );
}

function InstanceCompact({list}: {list: Instance[]}) {
    return (
        <div className="grid grid-cols-4 gap-2">
            {list.map((inst, i) => (
                <Card
                    key={i}
                    isPressable
                    className="px-3.5 py-3 flex flex-col gap-2 cursor-pointer border border-line"
                    style={cardSurfaceStyle}
                >
                    {/* Row 1 */}
                    <div className="flex items-center gap-1.5 min-w-0">
                        <div className="text-xs font-bold tracking-tight overflow-hidden text-ellipsis whitespace-nowrap flex-1 text-left">
                            {inst.name}
                        </div>
                        {inst.playing && (
                            <span
                                className="pulse-dot flex-shrink-0"
                                style={{width: 6, height: 6}}
                            />
                        )}
                        <Button
                            isIconOnly
                            variant="flat"
                            size="sm"
                            aria-label="More"
                            className="w-[22px] h-[22px] min-w-0 flex-shrink-0"
                        >
                            <I.more size={12}/>
                        </Button>
                    </div>
                    {/* Row 2 */}
                    <div className="font-mono flex items-center gap-1.5 text-[10px] text-ink-3">
                        <span>{inst.mc}</span>
                        <span className="opacity-50">·</span>
                        <Chip variant={inst.color} className="text-[9px] px-1.5 py-px">
                            {inst.loader}
                        </Chip>
                        <div className="flex-1"/>
                        <Button
                            color="success"
                            size="sm"
                            className="font-bold min-w-0 h-auto px-2 py-1 text-[10px]"
                            startContent={<I.play size={9}/>}
                        >
                            Play
                        </Button>
                    </div>
                </Card>
            ))}
        </div>
    );
}

function InstanceTable({list}: {list: Instance[]}) {
    const thClass =
        "text-left px-3.5 py-2.5 text-[10px] font-semibold tracking-[0.06em] uppercase text-ink-3 border-b border-line";
    const tdClass =
        "px-3.5 py-3 text-xs text-ink-1 align-middle border-b border-[color-mix(in_oklab,var(--line)_60%,transparent)]";
    return (
        <Card
            className="p-0 overflow-hidden border border-line"
            style={cardSurfaceStyle}
        >
            <table className="w-full border-collapse">
                <thead>
                <tr>
                    <th className={`${thClass} w-9`}></th>
                    <th className={thClass}>Instance</th>
                    <th className={thClass}>Loader</th>
                    <th className={thClass}>MC Version</th>
                    <th className={`${thClass} text-right`}>Mods</th>
                    <th className={`${thClass} text-right`}>Playtime</th>
                    <th className={thClass}>Last Played</th>
                    <th className={`${thClass} w-[120px]`}></th>
                </tr>
                </thead>
                <tbody>
                {list.map((inst, i) => (
                    <tr key={i} className="cursor-pointer">
                        <td className={`${tdClass} pr-0`}>
                            <div className="w-6 h-6 rounded-md overflow-hidden relative shadow-[inset_0_0_0_1px_rgba(255,255,255,0.08)]">
                                <Scene biome={inst.biome} seed={inst.seed}/>
                            </div>
                        </td>
                        <td className={tdClass}>
                            <div className="flex items-center gap-2">
                                <span className="font-semibold">{inst.name}</span>
                                {inst.playing && (
                                    <Chip variant="green" className="text-[9px] px-1.5 py-px">
                                        <span className="pulse-dot" style={{width: 4, height: 4}}/>
                                        playing
                                    </Chip>
                                )}
                            </div>
                        </td>
                        <td className={tdClass}>
                            <Chip variant={inst.color} className="text-[9px] px-[7px] py-0.5">
                                {inst.loader}
                            </Chip>
                        </td>
                        <td className={`${tdClass} font-mono text-ink-2 text-[11px]`}>{inst.mc}</td>
                        <td
                            className={`${tdClass} text-right font-mono text-[11px] ${inst.mods ? "text-ink-1" : "text-ink-3"}`}
                        >
                            {inst.mods || "—"}
                        </td>
                        <td className={`${tdClass} text-right font-mono text-[11px]`}>{inst.playtime}</td>
                        <td className={`${tdClass} text-ink-3 text-[11px]`}>{inst.lastPlayed}</td>
                        <td className={`${tdClass} text-right`}>
                            <div className="inline-flex gap-1">
                                <Button
                                    color="success"
                                    size="sm"
                                    className="font-bold text-[11px]"
                                    startContent={<I.play size={10}/>}
                                >
                                    Play
                                </Button>
                                <Button
                                    isIconOnly
                                    variant="bordered"
                                    size="sm"
                                    aria-label="More"
                                    className="w-[26px] h-[26px] min-w-0"
                                >
                                    <I.more size={12}/>
                                </Button>
                            </div>
                        </td>
                    </tr>
                ))}
                </tbody>
            </table>
        </Card>
    );
}
