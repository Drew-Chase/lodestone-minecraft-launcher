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

export default function Home() {
    const [viewMode, setViewMode] = useState<ViewMode>("grid");
    const [activeTab, setActiveTab] = useState<Tab>("Recent");

    const featured = instances[0];

    return (
        <div className="flex flex-col flex-1 min-w-0 min-h-0" style={{background: "var(--bg-0)"}}>
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
                    startContent={<I.plus size={14}/>}
                    style={{fontWeight: 700}}
                >
                    New Instance
                </Button>
            </TitleBar>

            <div
                style={{
                    flex: 1,
                    overflowY: "auto",
                    padding: "24px 28px 40px",
                }}
            >
                {/* Hero — currently playing */}
                <div
                    style={{
                        position: "relative",
                        height: 260,
                        borderRadius: 20,
                        overflow: "hidden",
                        marginBottom: 28,
                        boxShadow: "0 20px 40px -20px rgba(0,0,0,0.8)",
                    }}
                >
                    <Scene biome={featured.biome} seed={featured.seed}/>
                    <Particles count={12}/>
                    <div
                        style={{
                            position: "absolute",
                            inset: 0,
                            background:
                                "linear-gradient(90deg, rgba(8,9,10,0.92) 0%, rgba(8,9,10,0.6) 45%, transparent 80%)",
                            display: "flex",
                            alignItems: "flex-end",
                            padding: 32,
                        }}
                    >
                        <div style={{maxWidth: 500}}>
                            <div style={{display: "flex", alignItems: "center", gap: 8, marginBottom: 12}}>
                                <Chip variant="green">
                                    <span className="pulse-dot" style={{width: 6, height: 6}}/> NOW PLAYING
                                </Chip>
                                <Chip variant="violet">FABRIC 0.14.21</Chip>
                                <Chip>MC 1.20.1</Chip>
                            </div>
                            <div
                                style={{
                                    fontSize: 40,
                                    fontWeight: 800,
                                    letterSpacing: -1,
                                    lineHeight: 1,
                                    marginBottom: 8,
                                }}
                            >
                                {featured.name}
                            </div>
                            <div
                                style={{
                                    fontSize: 13,
                                    color: "var(--ink-2)",
                                    marginBottom: 20,
                                    lineHeight: 1.5,
                                }}
                            >
                                Climb the floating islands, battle Slider bosses, uncover a continent hanging in the
                                void.
                                <span style={{color: "var(--mc-green)", marginLeft: 6}}>
                  {featured.mods} mods
                </span>{" "}
                                · last played {featured.lastPlayed}
                            </div>
                            <div style={{display: "flex", gap: 10}}>
                                <Button
                                    color="success"
                                    size="lg"
                                    startContent={<I.play size={14}/>}
                                    style={{fontWeight: 700, padding: "0 24px"}}
                                >
                                    Resume Session
                                </Button>
                                <Button
                                    isIconOnly
                                    variant="bordered"
                                    aria-label="Open Folder"
                                    style={{width: 42, height: 42}}
                                >
                                    <I.folder size={16}/>
                                </Button>
                                <Button
                                    isIconOnly
                                    variant="bordered"
                                    aria-label="Configure"
                                    style={{width: 42, height: 42}}
                                >
                                    <I.settings size={16}/>
                                </Button>
                                <Button isIconOnly variant="bordered" aria-label="More"
                                        style={{width: 42, height: 42}}>
                                    <I.more size={16}/>
                                </Button>
                            </div>
                        </div>
                    </div>
                    {/* Stats pill */}
                    <div
                        className="font-mono"
                        style={{
                            position: "absolute",
                            top: 24,
                            right: 24,
                            fontSize: 11,
                            color: "var(--ink-2)",
                            padding: "6px 10px",
                            background: "rgba(0,0,0,0.55)",
                            backdropFilter: "blur(12px)",
                            borderRadius: 6,
                            border: "1px solid rgba(255,255,255,0.08)",
                            display: "flex",
                            alignItems: "center",
                            gap: 6,
                        }}
                    >
                        <I.cpu size={12}/> 2.1 GB / 6 GB · 58 FPS
                    </div>
                </div>

                {/* Quick actions */}
                <div
                    style={{
                        display: "grid",
                        gridTemplateColumns: "repeat(4, 1fr)",
                        gap: 12,
                        marginBottom: 32,
                    }}
                >
                    {quickActions.map((a, i) => {
                        const IconC = a.icon;
                        return (
                            <Card
                                key={i}
                                isPressable
                                style={{
                                    padding: "16px 18px",
                                    display: "flex",
                                    flexDirection: "row",
                                    gap: 14,
                                    alignItems: "center",
                                    background:
                                        "linear-gradient(180deg, rgba(255,255,255,0.035) 0%, rgba(255,255,255,0.015) 100%)",
                                    border: "1px solid var(--line)",
                                }}
                            >
                                <div
                                    style={{
                                        width: 40,
                                        height: 40,
                                        borderRadius: 10,
                                        flexShrink: 0,
                                        background: `color-mix(in oklab, ${a.color} 18%, transparent)`,
                                        display: "flex",
                                        alignItems: "center",
                                        justifyContent: "center",
                                        color: a.color,
                                        border: `1px solid color-mix(in oklab, ${a.color} 30%, transparent)`,
                                    }}
                                >
                                    <IconC size={18}/>
                                </div>
                                <div style={{minWidth: 0, textAlign: "left"}}>
                                    <div style={{fontSize: 13, fontWeight: 600}}>{a.label}</div>
                                    <div style={{fontSize: 11, color: "var(--ink-3)", marginTop: 2}}>
                                        {a.sub}
                                    </div>
                                </div>
                            </Card>
                        );
                    })}
                </div>

                {/* Section header */}
                <div style={{display: "flex", alignItems: "center", marginBottom: 14, gap: 12}}>
                    <div style={{fontSize: 16, fontWeight: 700, letterSpacing: -0.3}}>Your Instances</div>
                    <div style={{flex: 1}}/>
                    <div style={{display: "flex", gap: 6}}>
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

const quickActions = [
    {
        icon: I.plus,
        label: "New Instance",
        sub: "From modpack, CurseForge, or scratch",
        color: "var(--mc-green)",
    },
    {icon: I.download, label: "Import", sub: ".zip, .mrpack, CurseForge", color: "var(--cyan)"},
    {icon: I.server, label: "Join Server", sub: "Realms · hosted · friends", color: "var(--amber)"},
    {icon: I.users, label: "Co-op Sync", sub: "3 friends online now", color: "var(--violet)"},
] as const;

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
            className="font-sans"
            style={{
                padding: "6px 12px",
                fontSize: 12,
                fontWeight: 500,
                color: active ? "var(--bg-0)" : "var(--ink-2)",
                borderRadius: 12,
                cursor: "pointer",
                background: active ? "var(--mc-green)" : "transparent",
                border: "none",
                boxShadow: active
                    ? "0 0 0 1px rgba(34,255,132,0.6), 0 8px 20px -8px var(--mc-green-glow)"
                    : "none",
                transition: "all 0.12s",
            }}
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
        <div
            style={{
                display: "flex",
                padding: 3,
                gap: 2,
                background: "rgba(255,255,255,0.04)",
                border: "1px solid var(--line)",
                borderRadius: 10,
            }}
        >
            {opts.map((o) => {
                const IconC = o.icon;
                const active = mode === o.id;
                return (
                    <button
                        key={o.id}
                        onClick={() => onChange(o.id)}
                        title={`${o.label} view`}
                        className="font-sans"
                        style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 6,
                            padding: "6px 10px",
                            fontSize: 11,
                            fontWeight: 600,
                            border: "none",
                            borderRadius: 7,
                            cursor: "pointer",
                            background: active
                                ? "color-mix(in oklab, var(--mc-green) 18%, transparent)"
                                : "transparent",
                            color: active ? "var(--mc-green)" : "var(--ink-2)",
                            boxShadow: active
                                ? "inset 0 0 0 1px color-mix(in oklab, var(--mc-green) 40%, transparent)"
                                : "none",
                        }}
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
        <div style={{display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 16}}>
            {list.map((inst, i) => (
                <Card
                    key={i}
                    isPressable
                    style={{
                        cursor: "pointer",
                        background:
                            "linear-gradient(180deg, rgba(255,255,255,0.035) 0%, rgba(255,255,255,0.015) 100%)",
                        border: "1px solid var(--line)",
                        overflow: "hidden",
                        padding: 0,
                    }}
                >
                    <div style={{position: "relative", height: 140, width: "100%"}}>
                        <Scene biome={inst.biome} seed={inst.seed}/>
                        {inst.playing && (
                            <div style={{position: "absolute", top: 10, left: 10, zIndex: 2}}>
                                <Chip variant="green" style={{fontSize: 9, padding: "2px 6px"}}>
                                    <span className="pulse-dot" style={{width: 5, height: 5}}/>
                                    PLAYING
                                </Chip>
                            </div>
                        )}
                        <div style={{position: "absolute", top: 10, right: 10, zIndex: 2}}>
                            <Button
                                isIconOnly
                                size="sm"
                                variant="flat"
                                aria-label="More"
                                style={{
                                    width: 28,
                                    height: 28,
                                    minWidth: 0,
                                    background: "rgba(0,0,0,0.5)",
                                    backdropFilter: "blur(8px)",
                                }}
                            >
                                <I.more size={14}/>
                            </Button>
                        </div>
                        <div
                            style={{
                                position: "absolute",
                                inset: 0,
                                background:
                                    "linear-gradient(180deg, transparent 55%, rgba(0,0,0,0.7) 100%)",
                            }}
                        />
                        <div
                            style={{
                                position: "absolute",
                                bottom: 10,
                                left: 12,
                                right: 12,
                                display: "flex",
                                alignItems: "flex-end",
                                zIndex: 2,
                            }}
                        >
                            <div style={{minWidth: 0, flex: 1}}>
                                <div style={{fontSize: 14, fontWeight: 700, marginBottom: 2}}>
                                    {inst.name}
                                </div>
                                <div
                                    className="font-mono"
                                    style={{fontSize: 10, color: "var(--ink-2)"}}
                                >
                                    {inst.version}
                                </div>
                            </div>
                            <Button
                                color="success"
                                size="sm"
                                startContent={<I.play size={11}/>}
                                style={{fontWeight: 700}}
                            >
                                Play
                            </Button>
                        </div>
                    </div>
                    <div
                        style={{
                            padding: "12px 14px",
                            display: "flex",
                            alignItems: "center",
                            gap: 10,
                            fontSize: 11,
                            color: "var(--ink-3)",
                        }}
                    >
            <span style={{display: "flex", alignItems: "center", gap: 4}}>
              <I.clock size={11}/> {inst.playtime}
            </span>
                        <span>·</span>
                        <span style={{display: "flex", alignItems: "center", gap: 4}}>
              <I.box size={11}/> {inst.mods} mods
            </span>
                        <div style={{flex: 1}}/>
                        <Chip variant={inst.color} style={{fontSize: 9, padding: "2px 6px"}}>
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
        <div style={{display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 8}}>
            {list.map((inst, i) => (
                <Card
                    key={i}
                    isPressable
                    style={{
                        padding: "12px 14px",
                        display: "flex",
                        flexDirection: "column",
                        gap: 8,
                        cursor: "pointer",
                        background:
                            "linear-gradient(180deg, rgba(255,255,255,0.035) 0%, rgba(255,255,255,0.015) 100%)",
                        border: "1px solid var(--line)",
                    }}
                >
                    {/* Row 1 */}
                    <div style={{display: "flex", alignItems: "center", gap: 6, minWidth: 0}}>
                        <div
                            style={{
                                fontSize: 12,
                                fontWeight: 700,
                                letterSpacing: -0.2,
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                                whiteSpace: "nowrap",
                                flex: 1,
                                textAlign: "left",
                            }}
                        >
                            {inst.name}
                        </div>
                        {inst.playing && (
                            <span className="pulse-dot" style={{width: 6, height: 6, flexShrink: 0}}/>
                        )}
                        <Button
                            isIconOnly
                            variant="flat"
                            size="sm"
                            aria-label="More"
                            style={{width: 22, height: 22, minWidth: 0, flexShrink: 0}}
                        >
                            <I.more size={12}/>
                        </Button>
                    </div>
                    {/* Row 2 */}
                    <div
                        className="font-mono"
                        style={{
                            display: "flex",
                            alignItems: "center",
                            gap: 6,
                            fontSize: 10,
                            color: "var(--ink-3)",
                        }}
                    >
                        <span>{inst.mc}</span>
                        <span style={{opacity: 0.5}}>·</span>
                        <Chip variant={inst.color} style={{fontSize: 9, padding: "1px 6px"}}>
                            {inst.loader}
                        </Chip>
                        <div style={{flex: 1}}/>
                        <Button
                            color="success"
                            size="sm"
                            startContent={<I.play size={9}/>}
                            style={{
                                fontWeight: 700,
                                minWidth: 0,
                                height: "auto",
                                padding: "4px 8px",
                                fontSize: 10,
                            }}
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
    const th: React.CSSProperties = {
        textAlign: "left",
        padding: "10px 14px",
        fontSize: 10,
        fontWeight: 600,
        letterSpacing: "0.06em",
        textTransform: "uppercase",
        color: "var(--ink-3)",
        borderBottom: "1px solid var(--line)",
    };
    const td: React.CSSProperties = {
        padding: "12px 14px",
        fontSize: 12,
        color: "var(--ink-1)",
        borderBottom: "1px solid color-mix(in oklab, var(--line) 60%, transparent)",
        verticalAlign: "middle",
    };
    return (
        <Card
            style={{
                padding: 0,
                overflow: "hidden",
                background:
                    "linear-gradient(180deg, rgba(255,255,255,0.035) 0%, rgba(255,255,255,0.015) 100%)",
                border: "1px solid var(--line)",
            }}
        >
            <table style={{width: "100%", borderCollapse: "collapse"}}>
                <thead>
                <tr>
                    <th style={{...th, width: 36}}></th>
                    <th style={th}>Instance</th>
                    <th style={th}>Loader</th>
                    <th style={th}>MC Version</th>
                    <th style={{...th, textAlign: "right"}}>Mods</th>
                    <th style={{...th, textAlign: "right"}}>Playtime</th>
                    <th style={th}>Last Played</th>
                    <th style={{...th, width: 120}}></th>
                </tr>
                </thead>
                <tbody>
                {list.map((inst, i) => (
                    <tr key={i} style={{cursor: "pointer"}}>
                        <td style={{...td, paddingRight: 0}}>
                            <div
                                style={{
                                    width: 24,
                                    height: 24,
                                    borderRadius: 6,
                                    overflow: "hidden",
                                    position: "relative",
                                    boxShadow: "inset 0 0 0 1px rgba(255,255,255,0.08)",
                                }}
                            >
                                <Scene biome={inst.biome} seed={inst.seed}/>
                            </div>
                        </td>
                        <td style={td}>
                            <div style={{display: "flex", alignItems: "center", gap: 8}}>
                                <span style={{fontWeight: 600}}>{inst.name}</span>
                                {inst.playing && (
                                    <Chip variant="green" style={{fontSize: 9, padding: "1px 6px"}}>
                                        <span className="pulse-dot" style={{width: 4, height: 4}}/>
                                        playing
                                    </Chip>
                                )}
                            </div>
                        </td>
                        <td style={td}>
                            <Chip variant={inst.color} style={{fontSize: 9, padding: "2px 7px"}}>
                                {inst.loader}
                            </Chip>
                        </td>
                        <td
                            style={{
                                ...td,
                                fontFamily: "'JetBrains Mono', monospace",
                                color: "var(--ink-2)",
                                fontSize: 11,
                            }}
                        >
                            {inst.mc}
                        </td>
                        <td
                            style={{
                                ...td,
                                textAlign: "right",
                                fontFamily: "'JetBrains Mono', monospace",
                                fontSize: 11,
                                color: inst.mods ? "var(--ink-1)" : "var(--ink-3)",
                            }}
                        >
                            {inst.mods || "—"}
                        </td>
                        <td
                            style={{
                                ...td,
                                textAlign: "right",
                                fontFamily: "'JetBrains Mono', monospace",
                                fontSize: 11,
                            }}
                        >
                            {inst.playtime}
                        </td>
                        <td style={{...td, color: "var(--ink-3)", fontSize: 11}}>
                            {inst.lastPlayed}
                        </td>
                        <td style={{...td, textAlign: "right"}}>
                            <div style={{display: "inline-flex", gap: 4}}>
                                <Button
                                    color="success"
                                    size="sm"
                                    startContent={<I.play size={10}/>}
                                    style={{fontWeight: 700, fontSize: 11}}
                                >
                                    Play
                                </Button>
                                <Button
                                    isIconOnly
                                    variant="bordered"
                                    size="sm"
                                    aria-label="More"
                                    style={{width: 26, height: 26, minWidth: 0}}
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
