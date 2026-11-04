import {useEffect, useState} from "react";
import {Ic} from "./Icons.tsx";
import {useReleaseContext} from "../hooks/useRelease.ts";

const REPO_URL = "https://github.com/drew-chase/lodestone-minecraft-launcher";

interface Stats {
    stars: number;
    total_downloads: number;
    forks: number;
    open_issues: number;
    license: string | null;
    license_name: string | null;
    contributors: number;
}

export default function OpenSource() {
    const {available, downloadUrl} = useReleaseContext();
    const [stats, setStats] = useState<Stats | null>(null);

    useEffect(() => {
        fetch("/api/stats")
            .then(r => r.ok ? r.json() : null)
            .then(setStats)
            .catch(() => setStats(null));
    }, []);

    const licenseSpdx = stats?.license ?? "MIT";
    const licenseName = stats?.license_name ?? "MIT License";
    const contributors = stats?.contributors ?? 0;
    const forks = stats?.forks ?? 0;

    return (
        <section id="open-source">
            <div className="container">
                <div className="section-eyebrow">OPEN SOURCE</div>
                <h2 className="section-title">The launcher is free. Forever.</h2>
                <p className="section-lede">
                    The Lodestone launcher is {licenseSpdx}-licensed and developed publicly on GitHub.
                    No ads, no telemetry, no feature paywall — every mod, modpack, instance, and co-op feature works without an account.
                    Cloud sync is the one optional add-on that keeps the lights on.
                </p>
                <div className="oss-card">
                    <div>
                        <h3>$0<span style={{fontSize: 18, color: "var(--ink-3)", marginLeft: 6, fontWeight: 500}}>/ launcher</span></h3>
                        <p>
                            The full launcher experience — instances, modpacks, imports, co-op sync — yours, free.
                            Cross-device sync is a small subscription that funds the servers it runs on. Everything else stays open source.
                        </p>
                        <div style={{display: "flex", gap: 10, flexWrap: "wrap"}}>
                            {available ? (
                                <a className="btn btn-primary btn-lg" href={downloadUrl ?? "#download"}><Ic.download size={15}/> Download free</a>
                            ) : (
                                <span className="btn btn-primary btn-lg" style={{opacity: 0.7, cursor: "default"}}><Ic.download size={15}/> Coming Soon</span>
                            )}
                            <a className="btn btn-ghost btn-lg" href={REPO_URL} target="_blank" rel="noopener noreferrer"><Ic.heart size={13}/> Sponsor on GitHub</a>
                        </div>
                    </div>
                    <div className="oss-list">
                        <div className="oss-row">
                            <div className="oss-ic" style={{background: "rgba(34,255,132,0.15)", color: "var(--mc-green)"}}><Ic.code size={16}/></div>
                            <div style={{flex: 1}}>
                                <div className="lbl">Source on GitHub</div>
                                <div className="sub">github.com/drew-chase/lodestone-minecraft-launcher</div>
                            </div>
                        </div>
                        <div className="oss-row">
                            <div className="oss-ic" style={{background: "rgba(151,71,255,0.15)", color: "var(--violet)"}}><Ic.shield size={16}/></div>
                            <div style={{flex: 1}}>
                                <div className="lbl">{licenseName}</div>
                                <div className="sub">Use, fork, redistribute freely</div>
                            </div>
                        </div>
                        <div className="oss-row">
                            <div className="oss-ic" style={{background: "rgba(71,217,255,0.15)", color: "var(--cyan)"}}><Ic.users size={16}/></div>
                            <div style={{flex: 1}}>
                                <div className="lbl">{contributors > 0 ? `${contributors.toLocaleString()} contributors` : "Contributors"}</div>
                                <div className="sub">Open to contributions from anyone</div>
                            </div>
                        </div>
                        <div className="oss-row">
                            <div className="oss-ic" style={{background: "rgba(255,181,69,0.15)", color: "var(--amber)"}}><Ic.gitFork size={16}/></div>
                            <div style={{flex: 1}}>
                                <div className="lbl">{forks > 0 ? `${forks.toLocaleString()} forks` : "Forks"}</div>
                                <div className="sub">Fork it, build your own launcher</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}
