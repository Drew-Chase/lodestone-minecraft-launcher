import {useEffect, useState, type ReactNode} from "react";
import {Card} from "@heroui/react";
import Scene from "../shell/Scene";
import {I} from "../shell/icons";
import {cardSurfaceStyle, type Instance} from "../library/instances";
import type {Biome} from "../shell/Scene";

type Props = {
    instance: Instance;
};

const screenshotBiomes: Biome[] = ["end", "nether", "mushroom", "ocean", "cherry", "snow"];

const sessionStats: {k: string; v: string; color: string}[] = [
    {k: "Playtime", v: "12 min", color: "var(--mc-green)"},
    {k: "Total", v: "24h 12m", color: "var(--ink-0)"},
    {k: "Memory", v: "2.1 / 6 GB", color: "var(--cyan)"},
    {k: "Framerate", v: "58 fps", color: "var(--amber)"},
    {k: "Tick time", v: "18.2 ms", color: "var(--ink-0)"},
];

const playingWith: {n: string; c: string}[] = [
    {n: "pixelpete", c: "#ff5ec8"},
    {n: "cavesworn", c: "#47d9ff"},
    {n: "sundayknight", c: "#ffb545"},
];

// Wrapper to reuse the "glassy card" styling for the sub-cards in this tab.
function OverviewCard({
                          children,
                          className = "",
                      }: {
    children: ReactNode;
    className?: string;
}) {
    return (
        <Card
            className={`p-[18px] border border-line ${className}`}
            style={cardSurfaceStyle}
        >
            {children}
        </Card>
    );
}

// Overview tab — 2fr/1fr grid. Left: About card, optional install progress,
// screenshots preview grid. Right: Session stats card, Playing With card.
export default function OverviewTab({instance}: Props) {
    const [progress, setProgress] = useState(62);
    const installing = progress < 100;

    useEffect(() => {
        const t = setInterval(
            () => setProgress((p) => (p >= 100 ? 100 : p + 0.6)),
            100,
        );
        return () => clearInterval(t);
    }, []);

    return (
        <div className="grid gap-[18px]" style={{gridTemplateColumns: "2fr 1fr"}}>
            <div>
                <OverviewCard className="mb-4">
                    <div className="text-[0.8125rem] font-semibold mb-2.5">About</div>
                    <div className="text-xs text-ink-2 leading-relaxed">
                        A re-imagining of the classic Aether mod — soar between floating isles,
                        battle Slider bosses, and uncover the secrets of a continent suspended
                        in the void.
                    </div>
                </OverviewCard>

                {installing && (
                    <OverviewCard className="mb-4">
                        <div className="flex items-center gap-2.5 mb-2.5">
                            <I.download size={14} className="text-mc-green"/>
                            <div className="text-[0.8125rem] font-semibold">
                                Syncing mod updates
                            </div>
                            <div className="flex-1"/>
                            <div className="font-mono text-[0.6875rem] text-mc-green">
                                {Math.round(progress)}%
                            </div>
                        </div>
                        <div className="h-1.5 rounded-[3px] bg-[rgba(255,255,255,0.06)] overflow-hidden relative">
                            <div
                                className="shimmer h-full rounded-[3px] transition-[width] duration-300 ease-in-out"
                                style={{
                                    width: `${progress}%`,
                                    background:
                                        "linear-gradient(90deg, var(--mc-green-dim) 0%, var(--mc-green) 100%)",
                                }}
                            />
                        </div>
                        <div className="text-[0.6875rem] text-ink-3 mt-2 font-mono">
                            Downloading iris-mc{instance.mc}-1.7.1.jar · 12.4 MB/s
                        </div>
                    </OverviewCard>
                )}

                <div className="text-[0.8125rem] font-semibold mb-2.5">Screenshots</div>
                <div className="grid grid-cols-3 gap-2.5">
                    {screenshotBiomes.map((b, i) => (
                        <div key={i} className="h-[100px] rounded-[10px] overflow-hidden relative">
                            <Scene biome={b} seed={i * 9}/>
                            <div
                                className="absolute bottom-1.5 left-2 font-mono"
                                style={{
                                    fontSize: "0.5625rem",
                                    color: "rgba(255,255,255,0.7)",
                                }}
                            >
                                2026-04-{10 + i}_18.0{i}.png
                            </div>
                        </div>
                    ))}
                </div>
            </div>

            <div>
                <OverviewCard className="mb-3.5">
                    <div className="text-xs text-ink-3 mb-3 font-mono tracking-[0.05em]">
                        SESSION
                    </div>
                    {sessionStats.map((s, idx) => (
                        <div
                            key={s.k}
                            className="flex py-2 text-xs"
                            style={{
                                borderBottom:
                                    idx === sessionStats.length - 1 ? "none" : "1px solid var(--line)",
                            }}
                        >
                            <div className="text-ink-2 flex-1">{s.k}</div>
                            <div
                                className="font-mono font-semibold"
                                style={{color: s.color}}
                            >
                                {s.v}
                            </div>
                        </div>
                    ))}
                </OverviewCard>

                <OverviewCard>
                    <div className="text-xs text-ink-3 mb-3 font-mono tracking-[0.05em]">
                        PLAYING WITH
                    </div>
                    {playingWith.map((u) => (
                        <div
                            key={u.n}
                            className="flex items-center gap-2.5 py-1.5"
                        >
                            <div
                                className="w-6 h-6 rounded-md flex items-center justify-center text-[0.625rem] font-bold text-black"
                                style={{background: u.c}}
                            >
                                {u.n.slice(0, 2).toUpperCase()}
                            </div>
                            <div className="text-xs">{u.n}</div>
                            <div className="flex-1"/>
                            <span className="pulse-dot" style={{width: 6, height: 6}}/>
                        </div>
                    ))}
                </OverviewCard>

            </div>
        </div>
    );
}
