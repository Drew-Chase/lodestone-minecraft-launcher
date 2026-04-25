import {useEffect, useState} from "react";
import {Spinner} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {cardSurfaceStyle} from "../surfaces";
import {I} from "../shell/icons";
import type {ProjectVersion} from "../../types/content";
import {formatCount, formatSize, timeAgo} from "../../types/content";

interface VersionsTabProps {
    projectId: string;
    platform: string;
}

export default function VersionsTab({projectId, platform}: VersionsTabProps) {
    const [versions, setVersions] = useState<ProjectVersion[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [expandedId, setExpandedId] = useState<string | null>(null);

    useEffect(() => {
        setLoading(true);
        setError(null);
        invoke<ProjectVersion[]>("get_project_versions", {projectId, platform})
            .then(setVersions)
            .catch(e => setError(String(e)))
            .finally(() => setLoading(false));
    }, [projectId, platform]);

    if (loading) {
        return (
            <div className="flex items-center justify-center py-16">
                <Spinner size="lg" color="success"/>
            </div>
        );
    }

    if (error) {
        return (
            <div
                className="border border-line rounded-xl flex items-center gap-3"
                style={{...cardSurfaceStyle, padding: 16, borderColor: "rgba(255,80,80,0.2)", color: "#ff5050", fontSize: 12}}
            >
                <I.x size={14}/> {error}
            </div>
        );
    }

    if (versions.length === 0) {
        return (
            <div
                className="border border-line rounded-xl flex flex-col items-center justify-center gap-3"
                style={{...cardSurfaceStyle, padding: 48}}
            >
                <I.pkg size={32} style={{color: "var(--ink-4)"}}/>
                <span style={{fontSize: 13, color: "var(--ink-3)"}}>No versions available</span>
            </div>
        );
    }

    return (
        <div className="flex flex-col gap-2">
            {versions.map(v => (
                <VersionRow
                    key={v.id}
                    version={v}
                    expanded={expandedId === v.id}
                    onToggle={() => setExpandedId(expandedId === v.id ? null : v.id)}
                />
            ))}
        </div>
    );
}

const versionTypeBadge: Record<string, {bg: string; color: string}> = {
    Release: {bg: "rgba(34,255,132,0.12)", color: "var(--mc-green)"},
    Beta: {bg: "rgba(255,180,50,0.12)", color: "#ffb432"},
    Alpha: {bg: "rgba(255,80,80,0.12)", color: "#ff5050"},
};

function VersionRow({version: v, expanded, onToggle}: {
    version: ProjectVersion;
    expanded: boolean;
    onToggle: () => void;
}) {
    const badge = versionTypeBadge[v.version_type] ?? versionTypeBadge.Release;
    const primaryFile = v.files.find(f => f.primary) ?? v.files[0];

    return (
        <div
            className="border border-line rounded-xl overflow-hidden"
            style={cardSurfaceStyle}
        >
            {/* Header row */}
            <div
                className="flex items-center gap-3 cursor-pointer hover:bg-white/[0.02] transition-colors"
                style={{padding: "12px 16px"}}
                onClick={onToggle}
            >
                {/* Version type badge */}
                <span
                    style={{
                        padding: "3px 8px",
                        borderRadius: 6,
                        fontSize: 10,
                        fontWeight: 700,
                        background: badge.bg,
                        color: badge.color,
                        flexShrink: 0,
                    }}
                >
                    {v.version_type}
                </span>

                {/* Name + version number */}
                <div className="flex-1 min-w-0">
                    <div style={{fontSize: 13, fontWeight: 600}} className="truncate">
                        {v.name}
                    </div>
                    <div className="flex items-center gap-3 mt-0.5" style={{fontSize: 11, color: "var(--ink-3)"}}>
                        <span style={{fontFamily: "var(--mono)"}}>{v.version_number}</span>
                        <span>{timeAgo(v.date_published)}</span>
                        <span className="flex items-center gap-1">
                            <I.download size={10}/> {formatCount(v.downloads)}
                        </span>
                    </div>
                </div>

                {/* Loaders + MC versions */}
                <div className="flex items-center gap-1.5 flex-shrink-0 flex-wrap justify-end" style={{maxWidth: 200}}>
                    {v.loaders.slice(0, 3).map(l => (
                        <span
                            key={l}
                            style={{
                                padding: "2px 7px",
                                borderRadius: 4,
                                fontSize: 9,
                                fontWeight: 600,
                                background: "rgba(255,255,255,0.06)",
                                color: "var(--ink-2)",
                            }}
                        >
                            {l}
                        </span>
                    ))}
                    {v.game_versions.length > 0 && (
                        <span
                            style={{
                                padding: "2px 7px",
                                borderRadius: 4,
                                fontSize: 9,
                                fontWeight: 600,
                                background: "rgba(34,255,132,0.08)",
                                color: "var(--mc-green)",
                                fontFamily: "var(--mono)",
                            }}
                        >
                            {v.game_versions[0]}
                            {v.game_versions.length > 1 && ` +${v.game_versions.length - 1}`}
                        </span>
                    )}
                </div>

                {/* Download button for primary file */}
                {primaryFile?.url && (
                    <a
                        href={primaryFile.url}
                        onClick={e => e.stopPropagation()}
                        className="flex-shrink-0 flex items-center justify-center"
                        style={{
                            width: 32,
                            height: 32,
                            borderRadius: 8,
                            background: "rgba(34,255,132,0.1)",
                            color: "var(--mc-green)",
                            border: "1px solid rgba(34,255,132,0.25)",
                        }}
                    >
                        <I.download size={14}/>
                    </a>
                )}

                {/* Expand chevron */}
                <I.chevDown
                    size={14}
                    style={{
                        color: "var(--ink-3)",
                        flexShrink: 0,
                        transition: "transform 0.15s",
                        transform: expanded ? "rotate(180deg)" : undefined,
                    }}
                />
            </div>

            {/* Expanded details */}
            {expanded && (
                <div style={{borderTop: "1px solid var(--line)", padding: "12px 16px"}}>
                    {/* Game versions */}
                    {v.game_versions.length > 1 && (
                        <div className="mb-3">
                            <div style={{fontSize: 10, fontWeight: 700, color: "var(--ink-3)", textTransform: "uppercase", letterSpacing: 1, marginBottom: 6}}>
                                Game Versions
                            </div>
                            <div className="flex gap-1.5 flex-wrap">
                                {v.game_versions.map(gv => (
                                    <span
                                        key={gv}
                                        style={{
                                            padding: "2px 7px", borderRadius: 4, fontSize: 10,
                                            background: "rgba(255,255,255,0.04)", color: "var(--ink-2)",
                                            fontFamily: "var(--mono)",
                                        }}
                                    >
                                        {gv}
                                    </span>
                                ))}
                            </div>
                        </div>
                    )}

                    {/* Files */}
                    {v.files.length > 0 && (
                        <div className="mb-3">
                            <div style={{fontSize: 10, fontWeight: 700, color: "var(--ink-3)", textTransform: "uppercase", letterSpacing: 1, marginBottom: 6}}>
                                Files
                            </div>
                            <div className="flex flex-col gap-1">
                                {v.files.map((f, i) => (
                                    <div
                                        key={i}
                                        className="flex items-center gap-3"
                                        style={{fontSize: 11, padding: "4px 0"}}
                                    >
                                        <I.pkg size={12} style={{color: "var(--ink-4)", flexShrink: 0}}/>
                                        <span className="truncate flex-1" style={{color: "var(--ink-1)", fontFamily: "var(--mono)"}}>
                                            {f.filename}
                                        </span>
                                        <span style={{color: "var(--ink-3)", fontSize: 10, fontFamily: "var(--mono)", flexShrink: 0}}>
                                            {formatSize(f.size)}
                                        </span>
                                        {f.primary && (
                                            <span style={{fontSize: 9, color: "var(--mc-green)", fontWeight: 600, flexShrink: 0}}>
                                                Primary
                                            </span>
                                        )}
                                        {f.url && (
                                            <a
                                                href={f.url}
                                                style={{color: "var(--mc-green)", flexShrink: 0}}
                                            >
                                                <I.download size={12}/>
                                            </a>
                                        )}
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}

                    {/* Dependencies */}
                    {v.dependencies.length > 0 && (
                        <div className="mb-3">
                            <div style={{fontSize: 10, fontWeight: 700, color: "var(--ink-3)", textTransform: "uppercase", letterSpacing: 1, marginBottom: 6}}>
                                Dependencies
                            </div>
                            <div className="flex flex-col gap-1">
                                {v.dependencies.map((d, i) => (
                                    <div key={i} className="flex items-center gap-2" style={{fontSize: 11}}>
                                        <span
                                            style={{
                                                padding: "1px 6px", borderRadius: 4, fontSize: 9, fontWeight: 700,
                                                background: d.kind === "Required" ? "rgba(255,80,80,0.1)" : "rgba(255,255,255,0.04)",
                                                color: d.kind === "Required" ? "#ff5050" : "var(--ink-3)",
                                            }}
                                        >
                                            {d.kind}
                                        </span>
                                        <span style={{color: "var(--ink-2)", fontFamily: "var(--mono)"}}>
                                            {d.project_id || d.version_id || "Unknown"}
                                        </span>
                                    </div>
                                ))}
                            </div>
                        </div>
                    )}

                    {/* Changelog */}
                    {v.changelog && (
                        <div>
                            <div style={{fontSize: 10, fontWeight: 700, color: "var(--ink-3)", textTransform: "uppercase", letterSpacing: 1, marginBottom: 6}}>
                                Changelog
                            </div>
                            <div
                                style={{
                                    fontSize: 12, lineHeight: 1.6, color: "var(--ink-2)",
                                    maxHeight: 200, overflowY: "auto",
                                    padding: 12, borderRadius: 8,
                                    background: "rgba(0,0,0,0.2)",
                                    whiteSpace: "pre-wrap", wordBreak: "break-word",
                                }}
                            >
                                {v.changelog}
                            </div>
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}
