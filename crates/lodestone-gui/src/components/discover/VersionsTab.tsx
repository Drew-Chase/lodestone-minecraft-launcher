import {cardSurfaceStyle} from "../surfaces";
import {I} from "../shell/icons";

export default function VersionsTab() {
    return (
        <div
            className="border border-line rounded-xl flex flex-col items-center justify-center gap-3"
            style={{...cardSurfaceStyle, padding: 48}}
        >
            <I.pkg size={32} style={{color: "var(--ink-4)"}}/>
            <span style={{fontSize: 14, fontWeight: 600, color: "var(--ink-2)"}}>
                Version listing coming soon
            </span>
            <span style={{fontSize: 12, color: "var(--ink-3)", maxWidth: 340, textAlign: "center", lineHeight: 1.5}}>
                Per-version downloads, changelogs, and file details will be
                available once the version-listing API is wired into hopper-mc.
            </span>
        </div>
    );
}
