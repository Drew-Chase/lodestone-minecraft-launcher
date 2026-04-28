import {useEffect, useState} from "react";
import Logo from "./Logo.tsx";

import discoverImg from "../img/screens/discover.webp";
import libraryImg from "../img/screens/library.webp";

const REPO_URL = "https://github.com/drew-chase/lodestone-minecraft-launcher";

function formatCount(n: number): { value: string; suffix: string } {
    if (n >= 1_000_000) return {value: (n / 1_000_000).toFixed(1), suffix: "M"};
    if (n >= 1_000) return {value: (n / 1_000).toFixed(1), suffix: "k"};
    return {value: String(n), suffix: ""};
}

function StatsStrip() {
    const [stats, setStats] = useState<{ stars: number; total_downloads: number } | null>(null);

    useEffect(() => {
        fetch("/api/stats")
            .then(r => r.ok ? r.json() : null)
            .then(setStats)
            .catch(() => setStats(null));
    }, []);

    const downloads = stats ? formatCount(stats.total_downloads) : null;
    const stars = stats && stats.stars >= 1000 ? formatCount(stats.stars) : null;

    return (
        <div className="container" style={{marginTop: 80, marginBottom: 0}}>
            <div className="stats">
                <div className="stat">
                    {downloads
                        ? <div className="stat-num">{downloads.value}<small>{downloads.suffix}</small></div>
                        : <div className="stat-num">--</div>}
                    <div className="stat-lbl">Total downloads</div>
                </div>
                <div className="stat">
                    {stars
                        ? <a href={REPO_URL} target="_blank" rel="noopener noreferrer" className="stat-num">{stars.value}<small>{stars.suffix}</small></a>
                        : <a href={REPO_URL} target="_blank" rel="noopener noreferrer" className="stat-num stat-link">Star on GitHub</a>}
                    <div className="stat-lbl">GitHub stars</div>
                </div>
                <div className="stat">
                    <div className="stat-num">800<small>ms</small></div>
                    <div className="stat-lbl">Cold launch time</div>
                </div>
                <div className="stat">
                    <div className="stat-num">38<small>MB</small></div>
                    <div className="stat-lbl">Install footprint</div>
                </div>
            </div>
        </div>
    );
}

export default function Steps() {
    return (
        <section id="how">
            <div className="container">
                <div className="section-eyebrow">HOW IT WORKS</div>
                <h2 className="section-title">From download to running modpack in 90 seconds.</h2>
                <div className="steps">
                    <div className="step">
                        <div className="step-num">01 · INSTALL</div>
                        <h4>Download &amp; sign in</h4>
                        <p>One installer. Java is provisioned automatically. Sign in with Microsoft, offline, or skip.</p>
                        <div className="step-mock" style={{position: "relative", display: "flex", alignItems: "center", justifyContent: "center", flexDirection: "column", gap: 8}}>
                            <div style={{width: 50, height: 50, borderRadius: 12, background: "rgba(34,255,132,0.12)", border: "1px solid rgba(34,255,132,0.3)", display: "flex", alignItems: "center", justifyContent: "center", color: "var(--mc-green)"}}>
                                <Logo size={32}/>
                            </div>
                            <div style={{width: "70%", height: 6, borderRadius: 3, background: "rgba(255,255,255,0.06)", overflow: "hidden", position: "relative"}}>
                                <div style={{width: "74%", height: "100%", background: "var(--mc-green)", boxShadow: "0 0 8px var(--mc-green-glow)"}}/>
                            </div>
                            <div style={{fontSize: 9, fontFamily: "var(--mono)", color: "var(--mc-green)"}}>74% · 28.4 MB/s</div>
                        </div>
                    </div>
                    <div className="step">
                        <div className="step-num">02 · DISCOVER</div>
                        <h4>Pick a modpack</h4>
                        <p>Browse curated featured packs or search 18,000 community-built modpacks. One click installs everything.</p>
                        <div className="step-mock"><img src={discoverImg} alt="Discover modpacks" loading="lazy"/></div>
                    </div>
                    <div className="step">
                        <div className="step-num">03 · PLAY</div>
                        <h4>Hit launch</h4>
                        <p>Lodestone resolves dependencies, applies optimizations, and launches Minecraft — usually in under a second.</p>
                        <div className="step-mock"><img src={libraryImg} alt="Library home" loading="lazy"/></div>
                    </div>
                </div>
                <StatsStrip/>
            </div>
        </section>
    );
}
