import {Ic} from "./Icons.tsx";
import {useReleaseContext, pickPrimaryAsset} from "../hooks/useRelease.ts";
import DownloadButton from "./DownloadButton.tsx";

export default function FinalCTA() {
    const {available, loading, version, publishedAt, windows, macos, linux} = useReleaseContext();

    const fmtDate = (iso: string | null) => {
        if (!iso) return null;
        return new Date(iso).toLocaleDateString("en-US", {month: "long", day: "numeric", year: "numeric"});
    };

    const osBtn = (platform: ReturnType<typeof pickPrimaryAsset> extends infer T ? T : never, icon: typeof Ic.win, label: string, meta: string) => {
        const asset = platform;
        if (!available || !asset) {
            return (
                <span className="os-btn" style={{opacity: 0.5, cursor: "default"}}>
                    {icon({size: 16})}
                    <span><b>{label}</b><span className="os-meta"> · {meta}</span></span>
                </span>
            );
        }
        return (
            <a className="os-btn" href={asset.download_url}>
                {icon({size: 16})}
                <span><b>{label}</b><span className="os-meta"> · {meta}</span></span>
            </a>
        );
    };

    return (
        <section className="final-cta" id="download">
            <div className="container">
                <h2>Ready to upgrade your launcher?</h2>
                <p>Free. Open source. Available on every platform you'd actually use.</p>
                <div className="actions">
                    {loading ? (
                        <span className="btn btn-primary btn-lg" style={{opacity: 0.6, pointerEvents: "none"}}><Ic.download size={15}/> Loading...</span>
                    ) : available ? (
                        <DownloadButton/>
                    ) : (
                        <span className="btn btn-primary btn-lg" style={{opacity: 0.7, cursor: "default"}}><Ic.download size={15}/> Coming Soon</span>
                    )}
                </div>
                <div className="os-buttons">
                    {osBtn(pickPrimaryAsset(macos), Ic.apple, "macOS", "14+ · ARM64 + Intel")}
                    {osBtn(pickPrimaryAsset(windows), Ic.win, "Windows", "10+ · x64")}
                    {osBtn(pickPrimaryAsset(linux), Ic.linux, "Linux", ".deb · .rpm · AppImage")}
                </div>
                {available && (
                    <div style={{marginTop: 28, fontSize: 11, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                        v{version}{publishedAt ? ` · Released ${fmtDate(publishedAt)}` : ""} · <a href={`https://github.com/drew-chase/lodestone-minecraft-launcher/releases/tag/${version}`} style={{color: "var(--mc-green)"}} target="_blank" rel="noopener noreferrer">changelog</a>
                    </div>
                )}
                {!available && !loading && (
                    <div style={{marginTop: 28, fontSize: 11, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                        No releases yet — check back soon or star the repo to get notified.
                    </div>
                )}
            </div>
        </section>
    );
}
