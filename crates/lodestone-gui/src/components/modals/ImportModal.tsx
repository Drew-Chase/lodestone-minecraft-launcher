import {useState} from "react";
import ModalShell from "./ModalShell";
import {FooterBtn, Label} from "./primitives";
import {I} from "../shell/icons";

type Props = {isOpen: boolean; onClose: () => void};

type TabKey = "file" | "url" | "restore";

const tabs: {key: TabKey; label: string}[] = [
    {key: "file", label: "From file"},
    {key: "url", label: "From URL"},
    {key: "restore", label: "Restore backup"},
];

const recents = [
    {name: "Better MC [FABRIC] v27", size: "512 MB", source: "CurseForge", time: "2h ago"},
    {name: "Fabulously Optimized", size: "84 MB", source: ".mrpack", time: "yesterday"},
    {name: "All the Mods 10", size: "1.2 GB", source: "CurseForge", time: "3d ago"},
];

const fileTiles: {ext: string; rotate: number; translateY: number}[] = [
    {ext: ".zip", rotate: -5, translateY: 4},
    {ext: ".mrpack", rotate: 0, translateY: 0},
    {ext: ".mcpack", rotate: 5, translateY: 4},
];

export default function ImportModal({isOpen, onClose}: Props) {
    const [activeTab, setActiveTab] = useState<TabKey>("file");
    return (
        <ModalShell
            isOpen={isOpen}
            onClose={onClose}
            title="Import Instance"
            subtitle="From a modpack file, CurseForge URL, or backup archive."
            icon={I.download}
            accent="var(--cyan)"
            size="2xl"
            footer={
                <>
                    <FooterBtn onClick={onClose}>Cancel</FooterBtn>
                    <FooterBtn primary accent="var(--cyan)" onClick={onClose}>
                        Import
                    </FooterBtn>
                </>
            }
        >
            {/* Segmented tabs */}
            <div className="flex gap-1 p-1 rounded-[10px] bg-[rgba(0,0,0,0.3)] border border-line mb-[18px]">
                {tabs.map((t) => {
                    const active = activeTab === t.key;
                    return (
                        <button
                            key={t.key}
                            type="button"
                            onClick={() => setActiveTab(t.key)}
                            className="flex-1 px-3 py-2 rounded-[7px] text-xs font-semibold cursor-pointer transition-colors"
                            style={{
                                background: active
                                    ? "color-mix(in oklab, var(--cyan) 14%, transparent)"
                                    : "transparent",
                                color: active ? "var(--cyan)" : "var(--ink-2)",
                                border: active
                                    ? "1px solid color-mix(in oklab, var(--cyan) 30%, transparent)"
                                    : "1px solid transparent",
                            }}
                        >
                            {t.label}
                        </button>
                    );
                })}
            </div>

            {/* Drop zone */}
            <div
                className="py-[34px] px-5 rounded-[14px] text-center relative"
                style={{
                    border: "2px dashed color-mix(in oklab, var(--cyan) 40%, transparent)",
                    background:
                        "radial-gradient(ellipse at center, rgba(56,224,255,0.09), transparent 70%)",
                }}
            >
                {/* Splayed file-type tiles */}
                <div className="flex justify-center gap-3 mb-3.5">
                    {fileTiles.map((tile) => (
                        <div
                            key={tile.ext}
                            className="relative w-[60px] h-[70px] rounded-lg border border-line flex flex-col items-center justify-center shadow-[0_6px_20px_rgba(0,0,0,0.4)]"
                            style={{
                                background:
                                    "linear-gradient(180deg, rgba(40,46,42,0.9), rgba(26,30,28,0.9))",
                                transform: `rotate(${tile.rotate}deg) translateY(${tile.translateY}px)`,
                            }}
                        >
                            <div
                                className="font-mono text-accent-cyan font-bold tracking-[0.5px]"
                                style={{fontSize: "0.625rem"}}
                            >
                                {tile.ext}
                            </div>
                            <div
                                className="absolute bottom-2 w-6 h-0.5 rounded-sm"
                                style={{background: "var(--cyan)", opacity: 0.4}}
                            />
                            <div
                                className="absolute bottom-3.5 w-4 h-0.5 rounded-sm"
                                style={{background: "var(--cyan)", opacity: 0.25}}
                            />
                        </div>
                    ))}
                </div>
                <div className="text-sm font-semibold mb-1">Drop file here</div>
                <div className="text-xs text-ink-3">
                    or{" "}
                    <span className="text-accent-cyan underline cursor-pointer">
            browse your files
          </span>{" "}
                    · max 4 GB
                </div>
            </div>

            {/* Recent imports */}
            <div className="mt-5">
                <div className="flex items-center justify-between mb-2.5">
                    <Label className="!mb-0">Recent imports</Label>
                    <div className="text-[0.625rem] text-ink-3 font-mono">LAST 7 DAYS</div>
                </div>
                <div className="flex flex-col gap-1.5">
                    {recents.map((r, i) => (
                        <div
                            key={i}
                            className="px-3 py-2.5 rounded-lg bg-[rgba(255,255,255,0.02)] border border-line flex items-center gap-3"
                        >
                            <div
                                className="w-8 h-8 rounded-[7px] flex items-center justify-center flex-shrink-0 text-accent-cyan"
                                style={{background: "color-mix(in oklab, var(--cyan) 14%, transparent)"}}
                            >
                                <I.folder size={14}/>
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="text-xs font-semibold overflow-hidden text-ellipsis whitespace-nowrap">
                                    {r.name}
                                </div>
                                <div className="text-[0.625rem] text-ink-3 mt-0.5">
                                    {r.source} · {r.size} · {r.time}
                                </div>
                            </div>
                            <button
                                type="button"
                                className="bg-transparent border border-line text-ink-2 px-2.5 py-1 rounded-md text-[0.6875rem] cursor-pointer hover:bg-[rgba(255,255,255,0.04)]"
                            >
                                Reimport
                            </button>
                        </div>
                    ))}
                </div>
            </div>
        </ModalShell>
    );
}
