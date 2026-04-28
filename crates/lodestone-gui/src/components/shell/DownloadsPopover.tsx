import {useEffect, useRef, useState} from "react";
import {Popover, PopoverTrigger, PopoverContent, Tooltip, Button} from "@heroui/react";
import {I} from "./icons";
import Chip from "../Chip";
import {useLaunch, type InstallProgress} from "../../context/LaunchContext";

function relativeTime(ts: number): string {
    const diff = Date.now() - ts;
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return "just now";
    if (mins < 60) return `${mins}m ago`;
    return `${Math.floor(mins / 60)}h ago`;
}

function DLSection({label, count, children}: { label: string; count: number; children: React.ReactNode }) {
    return (
        <div className="mb-3">
            <div className="flex items-center gap-1.5 px-2 mb-1" style={{fontSize: 9, fontFamily: "var(--mono)", fontWeight: 700, letterSpacing: 1, color: "var(--ink-3)"}}>
                <span>{label}</span>
                <span style={{color: "var(--ink-4)"}}>{count}</span>
                <div className="flex-1 h-px bg-line"/>
            </div>
            {children}
        </div>
    );
}

function ActiveRow({p}: { p: InstallProgress }) {
    return (
        <div className="p-2.5 rounded-[10px] mb-1.5" style={{
            background: "rgba(34,255,132,0.04)",
            border: "1px solid color-mix(in oklab, var(--mc-green) 20%, transparent)",
        }}>
            <div className="flex items-center gap-2.5 mb-2">
                <div className="w-9 h-9 rounded-[7px] flex-shrink-0 bg-[color-mix(in_oklab,var(--mc-green)_14%,var(--bg-3))] border border-[color-mix(in_oklab,var(--mc-green)_30%,transparent)] flex items-center justify-center text-mc-green">
                    <I.download size={16}/>
                </div>
                <div className="flex-1 min-w-0">
                    <div className="text-xs font-bold truncate">{p.instanceName}</div>
                    <div className="text-[0.625rem] text-ink-3 font-mono">{p.stageLabel}</div>
                </div>
            </div>
            <div className="flex items-center gap-2">
                <div className="flex-1 h-1 rounded-sm bg-[rgba(255,255,255,0.06)] overflow-hidden relative">
                    <div
                        className="h-full bg-mc-green transition-[width] duration-300"
                        style={{width: `${p.progress * 100}%`, boxShadow: "0 0 8px var(--mc-green-glow)"}}
                    />
                </div>
                <div className="text-[0.625rem] font-mono text-ink-2 font-semibold min-w-[36px] text-right">
                    {Math.round(p.progress * 100)}%
                </div>
            </div>
            {p.filesTotal > 0 && (
                <div className="text-[0.625rem] font-mono mt-1.5 text-ink-3">
                    {p.filesDone} / {p.filesTotal} files
                </div>
            )}
        </div>
    );
}

export default function DownloadsPopoverButton() {
    const {installingInstances, completedInstalls, runningInstances, clearCompleted} = useLaunch();
    const activeCount = installingInstances.size;
    const hasActivity = activeCount > 0 || completedInstalls.length > 0;
    const [isOpen, setIsOpen] = useState(false);
    const prevCount = useRef(activeCount);

    // Auto-open when a new download starts
    useEffect(() => {
        if (activeCount > prevCount.current) {
            setIsOpen(true);
        }
        prevCount.current = activeCount;
    }, [activeCount]);

    // Calculate aggregate progress
    const activeList = [...installingInstances.values()];
    const aggProgress = activeList.length > 0
        ? activeList.reduce((sum, p) => sum + p.progress, 0) / activeList.length
        : 0;

    return (
        <Popover placement="right" offset={12} isOpen={isOpen} onOpenChange={setIsOpen}>
            <Tooltip content="Downloads" placement="right" delay={250} offset={14}>
                <div className="relative">
                    <PopoverTrigger>
                        <div className="nav-item">
                            <I.download size={20}/>
                            {activeCount > 0 && (
                                <div className="absolute -top-0.5 -right-0.5 w-3.5 h-3.5 rounded-full bg-mc-green flex items-center justify-center text-[8px] font-bold text-bg-0">
                                    {activeCount}
                                </div>
                            )}
                        </div>
                    </PopoverTrigger>
                </div>
            </Tooltip>
            <PopoverContent className="p-0 w-[380px] max-h-[560px] overflow-hidden flex flex-col bg-transparent border-0 shadow-none">
                <div className="w-full flex flex-col rounded-[14px] overflow-hidden" style={{
                    background: "linear-gradient(180deg, var(--bg-2) 0%, var(--bg-1) 100%)",
                    border: "1px solid var(--line-strong, var(--line))",
                    boxShadow: "0 20px 60px rgba(0,0,0,0.6), 0 0 0 1px rgba(34,255,132,0.08), 0 0 40px rgba(34,255,132,0.05)",
                }}>
                    {/* Header */}
                    <div className="p-3.5 border-b border-line" style={{background: "rgba(0,0,0,0.25)"}}>
                        <div className="flex items-center gap-2.5 mb-2.5">
                            <I.download size={15} className="text-mc-green"/>
                            <div className="text-[13px] font-bold">Downloads</div>
                            {activeCount > 0 && (
                                <Chip variant="green" className="text-[9px] px-[7px] py-0.5">
                                    <span className="pulse-dot" style={{width: 5, height: 5}}/> {activeCount} ACTIVE
                                </Chip>
                            )}
                        </div>
                        {activeCount > 0 && (
                            <div>
                                <div className="flex items-baseline gap-1.5 mb-1.5 font-mono text-[10px]">
                                    <span className="text-ink-2">{Math.round(aggProgress * 100)}%</span>
                                    <div className="flex-1"/>
                                </div>
                                <div className="h-1.5 rounded-[3px] bg-[rgba(255,255,255,0.05)] overflow-hidden relative">
                                    <div
                                        className="h-full rounded-[3px] transition-[width] duration-300"
                                        style={{
                                            width: `${aggProgress * 100}%`,
                                            background: "linear-gradient(90deg, var(--mc-green) 0%, color-mix(in oklab, var(--mc-green) 60%, var(--cyan)) 100%)",
                                            boxShadow: "0 0 10px var(--mc-green-glow)",
                                        }}
                                    />
                                </div>
                            </div>
                        )}
                    </div>

                    {/* Body */}
                    <div className="flex-1 overflow-y-auto p-3">
                        {!hasActivity && runningInstances.size === 0 && (
                            <div className="text-center py-8 text-ink-3 text-xs">
                                <I.download size={24} className="mx-auto mb-2 opacity-30"/>
                                No downloads
                            </div>
                        )}

                        {activeCount > 0 && (
                            <DLSection label="DOWNLOADING" count={activeCount}>
                                {activeList.map((p) => (
                                    <ActiveRow key={p.instanceId} p={p}/>
                                ))}
                            </DLSection>
                        )}

                        {runningInstances.size > 0 && (
                            <DLSection label="RUNNING" count={runningInstances.size}>
                                {[...runningInstances].map((id) => (
                                    <div key={id} className="flex items-center gap-2.5 px-2 py-2 rounded-lg">
                                        <div className="w-9 h-9 rounded-[7px] flex-shrink-0 bg-[color-mix(in_oklab,var(--mc-green)_14%,var(--bg-3))] border border-[color-mix(in_oklab,var(--mc-green)_30%,transparent)] flex items-center justify-center text-mc-green">
                                            <I.play size={14}/>
                                        </div>
                                        <div className="flex-1 min-w-0">
                                            <div className="text-xs font-semibold">Instance #{id}</div>
                                            <div className="text-[0.625rem] text-mc-green font-mono flex items-center gap-1">
                                                <span className="pulse-dot" style={{width: 5, height: 5}}/> Running
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </DLSection>
                        )}

                        {completedInstalls.length > 0 && (
                            <DLSection label="COMPLETED" count={completedInstalls.length}>
                                {completedInstalls.map((c) => (
                                    <div key={`${c.instanceId}-${c.completedAt}`} className="flex items-center gap-2.5 px-2 py-2 rounded-lg opacity-75">
                                        <div className="w-9 h-9 rounded-[7px] flex-shrink-0 bg-[color-mix(in_oklab,var(--mc-green)_14%,var(--bg-3))] border border-[color-mix(in_oklab,var(--mc-green)_30%,transparent)] flex items-center justify-center text-mc-green">
                                            <I.check size={14}/>
                                        </div>
                                        <div className="flex-1 min-w-0">
                                            <div className="text-xs font-semibold truncate">{c.instanceName}</div>
                                            <div className="text-[0.625rem] text-mc-green font-mono flex items-center gap-1">
                                                <I.check size={9}/> Installed · {relativeTime(c.completedAt)}
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </DLSection>
                        )}
                    </div>

                    {/* Footer */}
                    {completedInstalls.length > 0 && (
                        <div className="px-3.5 py-2.5 border-t border-line flex items-center" style={{background: "rgba(0,0,0,0.2)"}}>
                            <div className="flex-1"/>
                            <Button
                                variant="light"
                                size="sm"
                                className="text-[11px] text-ink-3"
                                onPress={clearCompleted}
                            >
                                Clear completed
                            </Button>
                        </div>
                    )}
                </div>
            </PopoverContent>
        </Popover>
    );
}
