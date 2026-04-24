import {useState, type ReactNode} from "react";
import {Button} from "@heroui/react";
import Chip from "../Chip";
import {Switch} from "../Switch";
import {I} from "../shell/icons";
import {cardSurfaceStyle} from "../surfaces";

// Per-instance settings — matches the v2 design's 180px sidebar + stacked
// sections layout with Danger Zone at the bottom. All controls are display-only.

type NavKey =
    | "java"
    | "window"
    | "launch"
    | "version"
    | "paths"
    | "advanced"
    | "danger";

type NavItem = {
    k: NavKey;
    label: string;
    icon: (p: {size?: number; className?: string}) => ReactNode;
    danger?: boolean;
};

const navItems: NavItem[] = [
    {k: "java", label: "Java & Memory", icon: I.cpu},
    {k: "window", label: "Window", icon: I.image},
    {k: "launch", label: "Launch Args", icon: I.terminal},
    {k: "version", label: "Game Version", icon: I.tag},
    {k: "paths", label: "Paths & Files", icon: I.folder},
    {k: "advanced", label: "Advanced", icon: I.settings},
    {k: "danger", label: "Danger Zone", icon: I.trash, danger: true},
];

export default function SettingsTab() {
    const [active, setActive] = useState<NavKey>("java");

    return (
        <div className="flex-1 overflow-y-auto px-7 pt-5 pb-10">
            <div className="grid gap-7" style={{gridTemplateColumns: "180px 1fr"}}>
                {/* Sidebar */}
                <div className="sticky top-0 self-start">
                {navItems.map((item) => {
                    const Icon = item.icon;
                    const isActive = active === item.k;
                    return (
                        <div
                            key={item.k}
                            onClick={() => setActive(item.k)}
                            className="flex items-center gap-2.5 px-2.5 py-2 rounded-md text-xs mb-0.5 cursor-pointer"
                            style={{
                                background: isActive ? "rgba(34,255,132,0.08)" : "transparent",
                                color: isActive
                                    ? "var(--mc-green)"
                                    : item.danger
                                        ? "#ff5a7a"
                                        : "var(--ink-2)",
                            }}
                        >
                            <Icon size={12}/>
                            <span>{item.label}</span>
                        </div>
                    );
                })}

                {/* Inherited callout */}
                <div className="mt-[18px] p-3 bg-bg-1 border border-line rounded-lg">
                    <div className="font-mono text-[0.625rem] text-ink-3 tracking-[0.5px] mb-1.5">
                        INHERITED
                    </div>
                    <div className="text-[0.6875rem] text-ink-2 leading-snug">
                        4 settings inherited from{" "}
                        <span className="text-mc-green">Global defaults</span>. Override
                        anything by toggling it here.
                    </div>
                </div>
            </div>

            {/* Main content — sections stack regardless of sidebar selection, the
                sidebar functions as in-page navigation (visual scroll target in the
                full app). */}
            <div>
                <Section
                    icon={I.cpu}
                    title="Java & Memory"
                    desc="Runtime selection and heap allocation for this instance."
                >
                    <Row label="Java runtime" hint="Auto-detected from your system installations.">
                        <SelectButton value="Temurin 17.0.9"/>
                    </Row>
                    <Row
                        label="Allocated memory"
                        hint="6 GB of 16 GB available · Minecraft recommends 4–8 GB for modded."
                    >
                        <div className="flex items-center gap-2.5 w-[280px]">
                            <div className="flex-1 relative h-1.5 bg-bg-2 rounded-[3px]">
                                <div
                                    className="absolute inset-0 bg-mc-green rounded-[3px]"
                                    style={{
                                        width: "37.5%",
                                        boxShadow: "0 0 8px rgba(34,255,132,0.5)",
                                    }}
                                />
                                <div
                                    className="absolute w-3.5 h-3.5 rounded-full bg-mc-green"
                                    style={{
                                        left: "37.5%",
                                        top: -4,
                                        transform: "translateX(-50%)",
                                        boxShadow: "0 0 0 3px var(--bg-0)",
                                    }}
                                />
                            </div>
                            <div className="font-mono text-xs font-semibold text-mc-green min-w-[42px] text-right">
                                6 GB
                            </div>
                        </div>
                    </Row>
                    <Row
                        label="Garbage collector"
                        hint="G1GC is recommended for most modpacks. Use ZGC for pause-sensitive gameplay."
                    >
                        <SelectButton value="G1GC"/>
                    </Row>
                    <Row label="Use system Java instead of bundled">
                        <Switch size="sm" color="success"/>
                    </Row>
                    <div className="py-3.5">
                        <div className="font-mono text-[0.6875rem] text-ink-3 mb-1.5">
                            JVM ARGUMENTS
                        </div>
                        <div
                            className="rounded-md border border-line p-2.5 font-mono text-[0.6875rem] leading-relaxed text-ink-1 max-h-[72px] overflow-hidden relative"
                            style={{background: "var(--bg-0)"}}
                        >
                            <span className="text-mc-green">-XX:+UseG1GC</span>{" "}
                            <span className="text-ink-2">-XX:+ParallelRefProcEnabled</span>{" "}
                            <span className="text-accent-amber">-XX:MaxGCPauseMillis=200</span>{" "}
                            <span className="text-ink-2">
                -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=30
              </span>{" "}
                            <span className="text-accent-cyan">
                -Dfml.ignoreInvalidMinecraftCertificates=true
              </span>
                        </div>
                    </div>
                </Section>

                <Section
                    icon={I.image}
                    title="Window"
                    desc="How Minecraft appears when this instance launches."
                >
                    <Row label="Launch mode">
                        <div className="flex gap-0.5 p-0.5 bg-bg-1 border border-line rounded-md">
                            {["Windowed", "Borderless", "Fullscreen"].map((m, i) => (
                                <div
                                    key={m}
                                    className={[
                                        "px-2.5 py-1 text-[0.6875rem] rounded-sm cursor-pointer",
                                        i === 0 ? "bg-bg-2 text-ink-0" : "bg-transparent text-ink-3",
                                    ].join(" ")}
                                >
                                    {m}
                                </div>
                            ))}
                        </div>
                    </Row>
                    <Row label="Resolution">
                        <div className="flex items-center gap-1.5">
                            <input
                                defaultValue="1920"
                                className="w-[72px] bg-bg-1 border border-line rounded-md px-2 py-1 text-[0.71875rem] text-ink-0 font-mono text-center outline-none"
                            />
                            <span className="text-ink-3 text-[0.6875rem]">×</span>
                            <input
                                defaultValue="1080"
                                className="w-[72px] bg-bg-1 border border-line rounded-md px-2 py-1 text-[0.71875rem] text-ink-0 font-mono text-center outline-none"
                            />
                        </div>
                    </Row>
                    <Row label="Hide launcher while playing" hint="Minimize Lodestone until you quit the game.">
                        <Switch defaultSelected size="sm" color="success"/>
                    </Row>
                    <Row label="Quit launcher after game exits">
                        <Switch size="sm" color="success"/>
                    </Row>
                </Section>

                <Section
                    icon={I.tag}
                    title="Game Version"
                    desc="Change Minecraft or mod-loader versions. Requires rebuild."
                >
                    <Row label="Minecraft">
                        <SelectButton value="1.20.1"/>
                    </Row>
                    <Row label="Mod loader">
                        <div className="flex items-center gap-2">
                            <Chip variant="violet" className="text-[0.625rem]">
                                FABRIC
                            </Chip>
                            <SelectButton value="0.14.21"/>
                        </div>
                    </Row>
                    <Row label="Include snapshots and pre-releases">
                        <Switch size="sm" color="success"/>
                    </Row>
                </Section>

                <Section
                    icon={I.settings}
                    title="Advanced"
                    desc="Power-user options. Change with care."
                >
                    <Row label="Environment variables" hint="Passed to the JVM process at launch.">
                        <Button variant="bordered" size="sm" className="text-[0.6875rem]">
                            3 set · Edit
                        </Button>
                    </Row>
                    <Row label="Pre-launch hook" hint="Run a shell command before the game starts.">
                        <Button
                            variant="bordered"
                            size="sm"
                            className="text-[0.6875rem]"
                            startContent={<I.plus size={10}/>}
                        >
                            Add script
                        </Button>
                    </Row>
                    <Row label="Verbose logging" hint="Emit DEBUG-level output to the console.">
                        <Switch defaultSelected size="sm" color="success"/>
                    </Row>
                    <Row label="Skip integrity check on launch">
                        <Switch size="sm" color="success"/>
                    </Row>
                </Section>

                    <DangerZone/>
                </div>
            </div>
        </div>
    );
}

// --- layout primitives ---

function Section({
                     icon: Icon,
                     title,
                     desc,
                     children,
                 }: {
    icon: (p: {size?: number; className?: string}) => ReactNode;
    title: string;
    desc: string;
    children: ReactNode;
}) {
    return (
        <div className="mb-7">
            <div className="flex items-start gap-3 mb-1">
                <div className="w-7 h-7 rounded-md bg-bg-1 border border-line flex items-center justify-center text-mc-green flex-shrink-0">
                    <Icon size={13}/>
                </div>
                <div className="flex-1">
                    <div className="text-[0.8125rem] font-semibold -tracking-[0.1px]">
                        {title}
                    </div>
                    <div className="text-[0.6875rem] text-ink-3 mt-0.5">{desc}</div>
                </div>
            </div>
            <div
                className="rounded-lg border border-line mt-3.5 px-[18px]"
                style={cardSurfaceStyle}
            >
                {children}
            </div>
        </div>
    );
}

// Row with a label/hint stack on the left and a right-aligned control slot.
// Each row adds a bottom border so successive rows form a divided list.
function Row({
                 label,
                 hint,
                 children,
             }: {
    label: string;
    hint?: string;
    children: ReactNode;
}) {
    return (
        <div className="flex items-center gap-4 py-3.5 border-b border-line last:border-b-0">
            <div className="flex-1 min-w-0">
                <div className={`text-[0.78125rem] text-ink-0 ${hint ? "mb-[3px]" : ""}`}>
                    {label}
                </div>
                {hint && (
                    <div className="text-[0.6875rem] text-ink-3 leading-snug">{hint}</div>
                )}
            </div>
            <div className="flex-shrink-0 flex items-center gap-2">{children}</div>
        </div>
    );
}

// Visual "select" — button with value + chevron. Static for this design pass.
function SelectButton({value}: {value: string}) {
    return (
        <button
            type="button"
            className="flex items-center gap-1.5 justify-between bg-bg-1 border border-line rounded-md px-2.5 py-[5px] text-[0.71875rem] text-ink-0 font-mono cursor-pointer min-w-[120px] hover:bg-bg-2"
        >
            <span>{value}</span>
            <I.chevDown size={10} className="text-ink-3"/>
        </button>
    );
}

// --- Danger Zone ---

function DangerZone() {
    const rows: {t: string; d: string; btn: string; danger?: boolean}[] = [
        {
            t: "Reinstall game files",
            d: "Redownload vanilla Minecraft jars for this version. Mods and worlds are preserved.",
            btn: "Reinstall",
        },
        {
            t: "Reset settings to defaults",
            d: "Revert this instance back to global defaults.",
            btn: "Reset",
        },
        {
            t: "Delete instance",
            d: "Permanently remove this instance and all of its worlds, mods, and screenshots.",
            btn: "Delete",
            danger: true,
        },
    ];

    return (
        <div>
            <div className="flex items-start gap-3 mb-1">
                <div
                    className="w-7 h-7 rounded-md bg-bg-1 border border-line flex items-center justify-center flex-shrink-0"
                    style={{color: "#ff5a7a"}}
                >
                    <I.trash size={13}/>
                </div>
                <div className="flex-1">
                    <div className="text-[0.8125rem] font-semibold -tracking-[0.1px]">
                        Danger Zone
                    </div>
                    <div className="text-[0.6875rem] text-ink-3 mt-0.5">
                        Destructive actions. These cannot be undone.
                    </div>
                </div>
            </div>
            <div
                className="rounded-lg p-[18px] mt-3.5 border"
                style={{
                    ...cardSurfaceStyle,
                    borderColor: "rgba(255,90,122,0.2)",
                }}
            >
                {rows.map((r, i) => (
                    <div
                        key={r.t}
                        className={`flex items-center gap-4 py-3 ${
                            i < rows.length - 1 ? "border-b border-line" : ""
                        }`}
                    >
                        <div className="flex-1">
                            <div
                                className="text-[0.78125rem] mb-[3px]"
                                style={{color: r.danger ? "#ff5a7a" : "var(--ink-0)"}}
                            >
                                {r.t}
                            </div>
                            <div className="text-[0.6875rem] text-ink-3 leading-snug">{r.d}</div>
                        </div>
                        <button
                            type="button"
                            className="px-3 py-1.5 text-[0.6875rem] rounded-md cursor-pointer border"
                            style={
                                r.danger
                                    ? {
                                        background: "rgba(255,90,122,0.1)",
                                        borderColor: "rgba(255,90,122,0.3)",
                                        color: "#ff5a7a",
                                    }
                                    : {
                                        background: "var(--bg-1)",
                                        borderColor: "var(--line)",
                                        color: "var(--ink-1)",
                                    }
                            }
                        >
                            {r.btn}
                        </button>
                    </div>
                ))}
            </div>
        </div>
    );
}
