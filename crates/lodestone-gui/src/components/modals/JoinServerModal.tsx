import ModalShell from "./ModalShell";
import {FooterBtn, Label, PingBars} from "./primitives";
import {I} from "../shell/icons";

type Props = {isOpen: boolean; onClose: () => void};

type Server = {
    name: string;
    addr: string;
    players: string;
    ping: number;
    motd: string;
    icon: string;
    online?: boolean;
};

const saved: Server[] = [
    {name: "Hypixel", addr: "mc.hypixel.net", players: "47,218", ping: 24, motd: "Largest MC server", icon: "#ffc93a"},
    {
        name: "Drew's Survival",
        addr: "192.168.1.42:25565",
        players: "3/8",
        ping: 8,
        motd: "Private · 1.21.4",
        icon: "#22ff84",
        online: true,
    },
    {name: "MineVille", addr: "play.mineville.org", players: "2,834", ping: 42, motd: "Factions · Skyblock", icon: "#9747ff"},
    {name: "CubeCraft", addr: "play.cubecraft.net", players: "12,901", ping: 38, motd: "Minigames", icon: "#38e0ff"},
];

// Drew's Survival is highlighted as the current/selected row in the design.
const SELECTED_INDEX = 1;

export default function JoinServerModal({isOpen, onClose}: Props) {
    return (
        <ModalShell
            isOpen={isOpen}
            onClose={onClose}
            title="Join Server"
            subtitle="Paste an address, pick a saved server, or hop into Realms."
            icon={I.server}
            accent="var(--amber)"
            size="3xl"
            footer={
                <>
                    <FooterBtn onClick={onClose}>Cancel</FooterBtn>
                    <FooterBtn primary accent="var(--amber)" onClick={onClose}>
                        Connect
                    </FooterBtn>
                </>
            }
        >
            {/* Address bar */}
            <Label>Server address</Label>
            <div className="flex gap-2">
                <div className="flex-1 relative">
          <span className="absolute left-3 top-1/2 -translate-y-1/2 text-accent-amber">
            <I.globe size={14}/>
          </span>
                    <input
                        defaultValue="mc.hypixel.net"
                        className="w-full pl-[34px] pr-3 py-[11px] rounded-[9px] bg-[rgba(0,0,0,0.3)] text-ink-0 text-[0.8125rem] font-mono outline-none box-border border"
                        style={{borderColor: "color-mix(in oklab, var(--amber) 22%, var(--line))"}}
                    />
                </div>
                <button
                    type="button"
                    className="px-3.5 py-[11px] rounded-[9px] bg-[rgba(255,255,255,0.04)] border border-line text-ink-1 text-xs font-semibold flex items-center gap-1.5 cursor-pointer hover:bg-[rgba(255,255,255,0.08)]"
                >
                    <I.zap size={13}/>
                    Test
                </button>
            </div>

            {/* Instance picker strip */}
            <div className="mt-3.5 px-3 py-2.5 rounded-[9px] bg-[rgba(255,255,255,0.02)] border border-line flex items-center gap-2.5">
                <div className="text-[0.6875rem] text-ink-3">Launch with:</div>
                <div
                    className="flex items-center gap-2 px-2.5 py-1 rounded-md bg-[rgba(34,255,132,0.08)] border"
                    style={{borderColor: "color-mix(in oklab, var(--mc-green) 22%, transparent)"}}
                >
                    <div
                        className="w-4 h-4 rounded-[3px]"
                        style={{background: "linear-gradient(135deg, #5d3e1e, #3a2612)"}}
                    />
                    <div className="text-[0.6875rem] font-semibold">Dragon Survival v2</div>
                    <div className="text-[0.625rem] text-ink-3 font-mono">1.21.4</div>
                </div>
                <div className="ml-auto text-[0.6875rem] text-accent-cyan cursor-pointer">
                    Change
                </div>
            </div>

            {/* Saved servers header */}
            <div className="mt-[22px] flex items-center justify-between mb-2.5">
                <Label className="!mb-0">Saved servers</Label>
                <div className="text-[0.6875rem] text-accent-amber cursor-pointer flex items-center gap-1">
                    <I.plus size={12}/> Add server
                </div>
            </div>

            {/* Saved servers list */}
            <div className="flex flex-col gap-1.5">
                {saved.map((s, i) => {
                    const selected = i === SELECTED_INDEX;
                    return (
                        <div
                            key={i}
                            className="px-3 py-[11px] rounded-[9px] flex items-center gap-3 cursor-pointer border"
                            style={{
                                background: selected
                                    ? "color-mix(in oklab, var(--mc-green) 8%, rgba(255,255,255,0.02))"
                                    : "rgba(255,255,255,0.02)",
                                borderColor: selected
                                    ? "color-mix(in oklab, var(--mc-green) 22%, transparent)"
                                    : "var(--line)",
                            }}
                        >
                            <div
                                className="relative w-9 h-9 rounded-[7px] flex-shrink-0"
                                style={{
                                    background: `linear-gradient(135deg, ${s.icon}, color-mix(in oklab, ${s.icon} 55%, #000))`,
                                }}
                            >
                                {s.online && (
                                    <div
                                        className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-mc-green border-2"
                                        style={{
                                            borderColor: "#181c1a",
                                            boxShadow: "0 0 8px var(--mc-green)",
                                        }}
                                    />
                                )}
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2">
                                    <div className="text-[0.8125rem] font-semibold">{s.name}</div>
                                    <div className="text-[0.625rem] text-ink-3 font-mono">{s.addr}</div>
                                </div>
                                <div className="text-[0.6875rem] text-ink-3 mt-0.5">{s.motd}</div>
                            </div>
                            <div className="flex flex-col items-end gap-0.5">
                                <div className="flex items-center gap-1">
                                    <PingBars ping={s.ping}/>
                                    <div className="text-[0.625rem] text-ink-3 font-mono">
                                        {s.ping}ms
                                    </div>
                                </div>
                                <div className="text-[0.625rem] text-ink-2 font-mono">
                                    {s.players}
                                </div>
                            </div>
                        </div>
                    );
                })}
            </div>
        </ModalShell>
    );
}
