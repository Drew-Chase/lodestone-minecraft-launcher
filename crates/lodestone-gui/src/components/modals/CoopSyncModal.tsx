import ModalShell from "./ModalShell";
import {FooterBtn, Label} from "./primitives";
import {I} from "../shell/icons";

type Props = {isOpen: boolean; onClose: () => void};

const stats = [
    {label: "FRIENDS ONLINE", val: "3", color: "var(--mc-green)"},
    {label: "INSTANCES SHARED", val: "12", color: "var(--violet)"},
    {label: "AVG SYNC TIME", val: "38s", color: "var(--cyan)"},
];

type Friend = {
    name: string;
    tag: string;
    sub: string;
    online: boolean;
    avatar: string;
    status: string;
};

const friends: Friend[] = [
    {
        name: "pix_mcrft",
        tag: "Playing Better MC",
        sub: "1.20.1 · Fabric · 86 mods",
        online: true,
        avatar: "#e08548",
        status: "IN NETHER",
    },
    {
        name: "Drew_builds",
        tag: "Playing All the Mods 10",
        sub: "1.21.1 · NeoForge · 412 mods",
        online: true,
        avatar: "#5a7fb3",
        status: "IN OVERWORLD",
    },
    {
        name: "stonebreaker",
        tag: "Idle in launcher",
        sub: "Vanilla 1.21.4",
        online: true,
        avatar: "#c16fa3",
        status: "BROWSING",
    },
];

const syncPlan = [
    {label: "MODS", add: 12, skip: 74},
    {label: "SHADERS", add: 1, skip: 0},
    {label: "RESOURCE PACKS", add: 2, skip: 1},
    {label: "CONFIGS", add: 8, skip: 0},
];

// The first friend is the currently-selected sync target in the design.
const SELECTED_FRIEND = 0;

export default function CoopSyncModal({isOpen, onClose}: Props) {
    return (
        <ModalShell
            isOpen={isOpen}
            onClose={onClose}
            title="Co-op Sync"
            subtitle="Mirror a friend's instance, match mods and shaders, then join their world."
            icon={I.users}
            accent="var(--violet)"
            size="2xl"
            footer={
                <>
                    <FooterBtn onClick={onClose}>Cancel</FooterBtn>
                    <FooterBtn primary accent="var(--violet)" onClick={onClose}>
                        Sync & join
                    </FooterBtn>
                </>
            }
        >
            {/* Stats */}
            <div className="grid grid-cols-3 gap-2.5 mb-[18px]">
                {stats.map((s, i) => (
                    <div
                        key={i}
                        className="px-3 py-2.5 rounded-[9px] bg-[rgba(255,255,255,0.02)] border border-line"
                    >
                        <div className="text-[0.625rem] text-ink-3 font-mono tracking-[0.05em]">
                            {s.label}
                        </div>
                        <div
                            className="text-xl font-bold mt-1"
                            style={{color: s.color}}
                        >
                            {s.val}
                        </div>
                    </div>
                ))}
            </div>

            <Label>Online now</Label>
            <div className="flex flex-col gap-2">
                {friends.map((f, i) => {
                    const selected = i === SELECTED_FRIEND;
                    return (
                        <div
                            key={i}
                            className="px-3.5 py-3 rounded-[11px] flex items-center gap-3.5 border"
                            style={{
                                background: selected
                                    ? "color-mix(in oklab, var(--violet) 9%, rgba(255,255,255,0.02))"
                                    : "rgba(255,255,255,0.02)",
                                borderColor: selected
                                    ? "color-mix(in oklab, var(--violet) 28%, transparent)"
                                    : "var(--line)",
                            }}
                        >
                            {/* Avatar with pixel face */}
                            <div className="relative flex-shrink-0">
                                <div
                                    className="relative w-11 h-11 rounded-[10px]"
                                    style={{
                                        background: `linear-gradient(135deg, ${f.avatar}, color-mix(in oklab, ${f.avatar} 50%, #000))`,
                                        boxShadow: selected
                                            ? `0 0 14px color-mix(in oklab, ${f.avatar} 45%, transparent)`
                                            : "none",
                                    }}
                                >
                                    {/* Pixel face */}
                                    <div
                                        className="absolute rounded-sm"
                                        style={{
                                            top: 12,
                                            left: 10,
                                            width: 6,
                                            height: 6,
                                            background: "#1a1a1a",
                                        }}
                                    />
                                    <div
                                        className="absolute rounded-sm"
                                        style={{
                                            top: 12,
                                            right: 10,
                                            width: 6,
                                            height: 6,
                                            background: "#1a1a1a",
                                        }}
                                    />
                                    <div
                                        className="absolute rounded-sm"
                                        style={{
                                            top: 24,
                                            left: 12,
                                            right: 12,
                                            height: 3,
                                            background: "#1a1a1a",
                                        }}
                                    />
                                </div>
                                {f.online && (
                                    <div
                                        className="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full bg-mc-green border-2"
                                        style={{
                                            borderColor: "#181c1a",
                                            boxShadow: "0 0 8px var(--mc-green)",
                                        }}
                                    />
                                )}
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2">
                                    <div className="text-[0.8125rem] font-bold">{f.name}</div>
                                    <div className="text-[0.5625rem] px-1.5 py-0.5 rounded-sm bg-[rgba(34,255,132,0.12)] text-mc-green font-mono font-semibold">
                                        {f.status}
                                    </div>
                                </div>
                                <div className="text-xs text-ink-1 mt-0.5">{f.tag}</div>
                                <div className="text-[0.6875rem] text-ink-3 mt-px font-mono">
                                    {f.sub}
                                </div>
                            </div>
                            {selected ? (
                                <button
                                    type="button"
                                    className="px-3.5 py-2 rounded-lg text-white text-xs font-semibold cursor-pointer flex items-center gap-1.5 border"
                                    style={{
                                        background: "var(--violet)",
                                        borderColor: "var(--violet)",
                                        boxShadow:
                                            "0 0 14px color-mix(in oklab, var(--violet) 40%, transparent)",
                                    }}
                                >
                                    <I.refresh size={12}/> Sync
                                </button>
                            ) : (
                                <button
                                    type="button"
                                    className="px-3 py-1.5 rounded-lg bg-transparent text-ink-1 text-xs font-semibold cursor-pointer border border-line hover:bg-[rgba(255,255,255,0.04)]"
                                >
                                    Mirror
                                </button>
                            )}
                        </div>
                    );
                })}
            </div>

            {/* Sync preview */}
            <div
                className="mt-[18px] p-3.5 rounded-[11px] border"
                style={{
                    background:
                        "linear-gradient(135deg, rgba(151,71,255,0.08), rgba(56,224,255,0.04))",
                    borderColor: "color-mix(in oklab, var(--violet) 22%, transparent)",
                }}
            >
                <div className="flex items-center gap-2 mb-2.5">
                    <I.refresh size={13} className="text-accent-violet"/>
                    <div className="text-xs font-semibold">
                        Sync plan for{" "}
                        <span style={{color: "var(--violet)"}}>pix_mcrft</span>
                    </div>
                </div>
                <div className="grid grid-cols-4 gap-2">
                    {syncPlan.map((p, i) => (
                        <div
                            key={i}
                            className="px-2.5 py-2 rounded-[7px] bg-[rgba(0,0,0,0.3)] border border-line"
                        >
                            <div className="text-[0.625rem] text-ink-3 font-mono tracking-[0.05em]">
                                {p.label}
                            </div>
                            <div className="text-[0.8125rem] font-bold mt-0.5">
                <span className="text-mc-green">
                  +{p.add}
                </span>
                                {p.skip > 0 && (
                                    <span className="text-ink-3 text-[0.625rem] ml-1 font-medium">
                    · {p.skip} kept
                  </span>
                                )}
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </ModalShell>
    );
}
