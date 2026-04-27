import {useEffect, useState} from "react";
import {Spinner} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {cardSurfaceStyle} from "../surfaces";
import {I} from "../shell/icons";
import type {ContentItem, Dependency, ProjectVersion} from "../../types/content";

interface DependenciesTabProps {
    deps: Dependency[];
    projectId?: string;
    platform?: string;
    onOpenDetail?: (item: ContentItem) => void;
}

interface ResolvedDep {
    projectId: string;
    item: ContentItem | null;
    versionId?: string;
    kind: string;
    name: string | null;
    iconUrl: string | null;
    versions: string[]; // which versions of the parent require this
}

const kindColors: Record<string, { bg: string; color: string }> = {
    Required: {bg: "rgba(255,80,80,0.1)", color: "#ff5050"},
    Optional: {bg: "rgba(255,180,50,0.1)", color: "#ffb432"},
    Incompatible: {bg: "rgba(255,80,80,0.08)", color: "#ff5050"},
    Embedded: {bg: "rgba(255,255,255,0.04)", color: "var(--ink-3)"},
};

export default function DependenciesTab({deps, projectId, platform, onOpenDetail}: DependenciesTabProps) {
    const [resolved, setResolved] = useState<ResolvedDep[]>([]);
    const [loading, setLoading] = useState(false);

    useEffect(() => {
        if (!projectId || !platform) {
            // Fall back to static deps from props
            setResolved(deps.map(d => ({
                projectId: d.project_id || "",
                item: null,
                versionId: d.version_id || undefined,
                kind: d.kind,
                name: null,
                iconUrl: null,
                versions: [],
            })));
            return;
        }

        setLoading(true);
        (async () => {
            try {
                // Fetch all versions to get per-version deps
                const versions = await invoke<ProjectVersion[]>("get_project_versions", {projectId, platform});

                // Aggregate: for each unique dep project_id, collect which versions require it
                const depMap = new Map<string, ResolvedDep>();
                for (const v of versions) {
                    for (const d of v.dependencies) {
                        if (!d.project_id) continue;
                        const key = `${d.project_id}-${d.kind}`;
                        if (!depMap.has(key)) {
                            depMap.set(key, {
                                projectId: d.project_id,
                                item: null,
                                versionId: d.version_id || undefined,
                                kind: d.kind,
                                name: null,
                                iconUrl: null,
                                versions: [],
                            });
                        }
                        const entry = depMap.get(key)!;
                        const vLabel = v.version_number || v.name;
                        if (!entry.versions.includes(vLabel)) {
                            entry.versions.push(vLabel);
                        }
                    }
                }

                // Resolve project names by fetching each dependency's content
                const entries = [...depMap.values()];
                await Promise.all(entries.map(async (entry) => {
                    try {
                        const item = await invoke<ContentItem | null>("get_content", {
                            id: entry.projectId,
                            platform,
                            contentType: "mod",
                        });
                        if (item) {
                            entry.name = item.title;
                            entry.iconUrl = item.icon_url || null;
                            entry.item = item;
                        }
                    } catch { /* leave null */ }
                }));

                // Sort: Required first, then Optional, then others
                const kindOrder: Record<string, number> = {Required: 0, Optional: 1, Embedded: 2, Incompatible: 3};
                entries.sort((a, b) => (kindOrder[a.kind] ?? 9) - (kindOrder[b.kind] ?? 9));

                setResolved(entries);
            } catch {
                // Fall back to props
                setResolved(deps.map(d => ({
                    projectId: d.project_id || "",
                    versionId: d.version_id || undefined,
                    kind: d.kind,
                    item: null,
                    name: null,
                    iconUrl: null,
                    versions: [],
                })));
            } finally {
                setLoading(false);
            }
        })();
    }, [projectId, platform]); // eslint-disable-line react-hooks/exhaustive-deps

    if (loading) {
        return (
            <div className="flex items-center justify-center py-16">
                <Spinner size="lg" color="success"/>
            </div>
        );
    }

    if (resolved.length === 0) {
        return (
            <div
                className="border border-line rounded-xl flex flex-col items-center justify-center gap-3"
                style={{...cardSurfaceStyle, padding: 48}}
            >
                <I.pkg size={32} style={{color: "var(--ink-4)"}}/>
                <span style={{fontSize: 13, color: "var(--ink-3)"}}>
                    No dependencies found
                </span>
            </div>
        );
    }

    return (
        <div className="flex flex-col gap-2">
            {resolved.map((d, i) => {
                const colors = kindColors[d.kind] ?? kindColors.Embedded;
                return (
                    <div
                        key={`${d.projectId}-${d.kind}-${i}`}
                        className={`border border-line rounded-xl overflow-hidden ${d.item && onOpenDetail ? "cursor-pointer hover:border-[rgba(34,255,132,0.3)]" : ""}`}
                        style={cardSurfaceStyle}
                        onClick={d.item && onOpenDetail ? () => onOpenDetail(d.item!) : undefined}
                    >
                        <div className="flex items-center gap-3" style={{padding: "12px 16px"}}>
                            {/* Icon */}
                            {d.iconUrl ? (
                                <img src={d.iconUrl} alt="" className="w-9 h-9 rounded-lg flex-shrink-0 object-cover"/>
                            ) : (
                                <div className="w-9 h-9 rounded-lg flex-shrink-0 flex items-center justify-center border border-line bg-[rgba(255,255,255,0.04)] text-ink-3">
                                    <I.box size={16}/>
                                </div>
                            )}

                            {/* Name + project ID */}
                            <div className="flex-1 min-w-0">
                                <div style={{fontSize: 13, fontWeight: 600}} className="truncate">
                                    {d.name || d.projectId}
                                </div>
                                {d.name && (
                                    <div style={{fontSize: 10, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                                        {d.projectId}
                                    </div>
                                )}
                            </div>

                            {/* Kind badge */}
                            <span style={{
                                padding: "3px 10px", borderRadius: 6, fontSize: 10, fontWeight: 700,
                                background: colors.bg, color: colors.color, flexShrink: 0,
                            }}>
                                {d.kind}
                            </span>
                        </div>

                        {/* Which versions require this dep */}
                        {d.versions.length > 0 && (
                            <div style={{padding: "0 16px 10px"}}>
                                <div style={{fontSize: 10, color: "var(--ink-4)", fontFamily: "var(--mono)", marginBottom: 4}}>
                                    Required by {d.versions.length} version{d.versions.length !== 1 ? "s" : ""}
                                </div>
                                <div className="flex gap-1 flex-wrap">
                                    {d.versions.slice(0, 8).map(v => (
                                        <span key={v} style={{
                                            padding: "2px 6px", borderRadius: 4, fontSize: 9,
                                            background: "rgba(255,255,255,0.04)", color: "var(--ink-2)",
                                            fontFamily: "var(--mono)",
                                        }}>
                                            {v}
                                        </span>
                                    ))}
                                    {d.versions.length > 8 && (
                                        <span style={{fontSize: 9, color: "var(--ink-3)", fontFamily: "var(--mono)", padding: "2px 4px"}}>
                                            +{d.versions.length - 8} more
                                        </span>
                                    )}
                                </div>
                            </div>
                        )}
                    </div>
                );
            })}
        </div>
    );
}
