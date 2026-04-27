import {useEffect, useState, type ReactNode} from "react";
import {Card} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {I} from "../shell/icons";
import {cardSurfaceStyle, type Instance} from "../library/instances";

type Props = {
    instance: Instance;
};

interface InstanceDetails {
    diskSizeBytes: number;
    fileCount: number;
    modCount: number;
    worldCount: number;
    screenshotCount: number;
}

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatDate(iso: string): string {
    if (!iso) return "—";
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, {year: "numeric", month: "short", day: "numeric"});
}

function InfoCard({children, className = ""}: { children: ReactNode; className?: string }) {
    return (
        <Card className={`p-[18px] border border-line ${className}`} style={cardSurfaceStyle}>
            {children}
        </Card>
    );
}

function StatRow({label, value, color}: { label: string; value: string; color?: string }) {
    return (
        <div className="flex py-2.5 text-xs border-b border-line last:border-b-0">
            <div className="text-ink-2 flex-1">{label}</div>
            <div className="font-mono font-semibold" style={{color: color ?? "var(--ink-0)"}}>
                {value}
            </div>
        </div>
    );
}

export default function OverviewTab({instance}: Props) {
    const [details, setDetails] = useState<InstanceDetails | null>(null);

    useEffect(() => {
        invoke<InstanceDetails>("get_instance_details", {instancePath: instance.instancePath})
            .then(setDetails)
            .catch(() => setDetails(null));
    }, [instance.instancePath]);

    return (
        <div className="flex-1 overflow-y-auto px-7 pt-5 pb-10">
            <div className="grid gap-[18px]" style={{gridTemplateColumns: "2fr 1fr"}}>
                {/* Left column */}
                <div>
                    <InfoCard className="mb-4">
                        <div className="text-xs text-ink-3 mb-3 font-mono tracking-[0.05em]">
                            INSTANCE INFO
                        </div>
                        <StatRow label="Minecraft Version" value={instance.mc} color="var(--mc-green)"/>
                        <StatRow
                            label="Mod Loader"
                            value={
                                instance.loader === "Vanilla"
                                    ? "Vanilla"
                                    : `${instance.loader} ${instance.loaderVersion ?? ""}`
                            }
                            color="var(--violet)"
                        />
                        <StatRow
                            label="Java Version"
                            value={instance.javaVersion ? `Java ${instance.javaVersion}` : "Auto"}
                        />
                        <StatRow label="Created" value={formatDate(instance.createdAt)}/>
                        <StatRow label="Last Played" value={instance.lastPlayed}/>
                        <StatRow
                            label="Disk Size"
                            value={details ? formatBytes(details.diskSizeBytes) : "…"}
                            color="var(--cyan)"
                        />
                    </InfoCard>

                    <InfoCard>
                        <div className="text-xs text-ink-3 mb-3 font-mono tracking-[0.05em]">
                            INSTANCE PATH
                        </div>
                        <div className="text-xs text-ink-1 font-mono break-all leading-relaxed">
                            {instance.instancePath}
                        </div>
                    </InfoCard>
                </div>

                {/* Right column */}
                <div>
                    <InfoCard>
                        <div className="text-xs text-ink-3 mb-3 font-mono tracking-[0.05em]">
                            QUICK STATS
                        </div>
                        {[
                            {
                                icon: I.box,
                                label: "Mods",
                                value: details?.modCount ?? 0,
                                color: "var(--violet)",
                            },
                            {
                                icon: I.globe,
                                label: "Worlds",
                                value: details?.worldCount ?? 0,
                                color: "var(--cyan)",
                            },
                            {
                                icon: I.image,
                                label: "Screenshots",
                                value: details?.screenshotCount ?? 0,
                                color: "var(--amber)",
                            },
                            {
                                icon: I.hardDrive,
                                label: "Total Files",
                                value: details?.fileCount ?? 0,
                                color: "var(--ink-0)",
                            },
                        ].map((s) => {
                            const Icon = s.icon;
                            return (
                                <div
                                    key={s.label}
                                    className="flex items-center gap-3 py-2.5 border-b border-line last:border-b-0"
                                >
                                    <div
                                        className="w-7 h-7 rounded-md flex items-center justify-center border border-line"
                                        style={{
                                            background: `color-mix(in oklab, ${s.color} 12%, transparent)`,
                                            color: s.color,
                                        }}
                                    >
                                        <Icon size={13}/>
                                    </div>
                                    <div className="flex-1 text-xs text-ink-2">{s.label}</div>
                                    <div className="font-mono text-sm font-semibold" style={{color: s.color}}>
                                        {s.value}
                                    </div>
                                </div>
                            );
                        })}
                    </InfoCard>
                </div>
            </div>
        </div>
    );
}
