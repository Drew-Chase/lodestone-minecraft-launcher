import {cardSurfaceStyle} from "../surfaces";
import {I} from "../shell/icons";
import type {Dependency} from "../../types/content";

export default function DependenciesTab({deps}: {deps: Dependency[]}) {
    if (deps.length === 0) {
        return (
            <div
                className="border border-line rounded-xl flex flex-col items-center justify-center gap-3"
                style={{...cardSurfaceStyle, padding: 48}}
            >
                <I.pkg size={32} style={{color: "var(--ink-4)"}}/>
                <span style={{fontSize: 13, color: "var(--ink-3)"}}>
                    No dependencies reported
                </span>
            </div>
        );
    }

    return (
        <div className="border border-line rounded-xl overflow-hidden" style={cardSurfaceStyle}>
            {deps.map((d, i) => (
                <div
                    key={i}
                    className="flex items-center gap-3"
                    style={{
                        padding: "10px 16px",
                        borderBottom: i < deps.length - 1 ? "1px solid var(--line)" : "none",
                    }}
                >
                    <I.pkg size={14} style={{color: "var(--ink-3)"}}/>
                    <span style={{fontSize: 12, color: "var(--ink-1)"}}>
                        {d.project_id || d.version_id || "Unknown"}
                    </span>
                    <span
                        style={{
                            marginLeft: "auto",
                            fontSize: 10,
                            fontFamily: "var(--mono)",
                            fontWeight: 600,
                            padding: "2px 8px",
                            borderRadius: 999,
                            background: d.kind === "Required"
                                ? "rgba(34,255,132,0.1)"
                                : "rgba(255,255,255,0.04)",
                            color: d.kind === "Required"
                                ? "var(--mc-green)"
                                : "var(--ink-3)",
                        }}
                    >
                        {d.kind}
                    </span>
                </div>
            ))}
        </div>
    );
}
