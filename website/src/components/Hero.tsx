import {useEffect, useState} from "react";
import {Ic} from "./Icons.tsx";
import {useReleaseContext, formatBytes} from "../hooks/useRelease.ts";
import DownloadButton from "./DownloadButton.tsx";

import libraryImg from "../img/screens/library.webp";
import discoverImg from "../img/screens/discover.webp";
import instanceImg from "../img/screens/instance-page.webp";
import syncImg from "../img/screens/sync-modal.webp";
import newInstanceImg from "../img/screens/new-instance-modal.webp";

function HeroShowcaseReal() {
    const screens = [
        {src: libraryImg, label: "lodestone://library"},
        {src: discoverImg, label: "lodestone://discover"},
        {src: instanceImg, label: "lodestone://instance/Survival"},
        {src: syncImg, label: "lodestone://co-op-sync"},
        {src: newInstanceImg, label: "lodestone://new-instance"},
    ];
    const [idx, setIdx] = useState(0);
    const [paused, setPaused] = useState(false);
    useEffect(() => {
        if (paused) return;
        const id = setInterval(() => setIdx(i => (i + 1) % screens.length), 4500);
        return () => clearInterval(id);
    }, [paused, screens.length]);
    return (
        <div className="hero-showcase" onMouseEnter={() => setPaused(true)} onMouseLeave={() => setPaused(false)}>
            <div className="showcase-window">
                <div className="showcase-chrome">
                    <div className="dots"><span/><span/><span/></div>
                    <div className="url">{screens[idx].label}</div>
                    <div style={{width: 40}}/>
                </div>
                <div className="showcase-screen shot-screen">
                    {screens.map((s, i) => (
                        <img key={s.src} src={s.src} alt={s.label} loading={i === 0 ? "eager" : "lazy"}
                             className={`shot-frame ${i === idx ? "active" : ""}`}/>
                    ))}
                    <div className="shot-dots">
                        {screens.map((s, i) => (
                            <button key={s.src} onClick={() => setIdx(i)}
                                    className={i === idx ? "on" : ""} aria-label={s.label}/>
                        ))}
                    </div>
                </div>
            </div>
        </div>
    );
}

export default function Hero() {
    const {available, version, installSize} = useReleaseContext();

    return (
        <section className="hero" id="top">
            <div className="hero-grid"/>
            <div className="hero-aurora"/>
            <div className="container hero-inner">
                {available && (
                    <div className="hero-tag">
                        <span className="pulse"/>
                        v{version} · NOW AVAILABLE
                    </div>
                )}
                <h1>
                    The Minecraft launcher<br/>
                    you've been <span className="accent">crafting toward.</span>
                </h1>
                <p className="lede">
                    Lodestone is a fast, beautiful, open-source launcher for Java Edition.
                    Browse 50,000+ mods from Modrinth and CurseForge, manage instances,
                    import any modpack, and sync instances with friends — all in one place.
                </p>
                <div className="hero-cta">
                    <DownloadButton/>
                    <a className="btn btn-ghost btn-lg" href="#demo">
                        <Ic.play size={13}/> Watch the demo
                    </a>
                </div>
                <div className="hero-meta">
                    <span><Ic.shield size={11}/> Verified builds · code-signed</span>
                    <span>·</span>
                    <span><Ic.heart size={11}/> Free forever · MIT license</span>
                    {available && (
                        <>
                            <span>·</span>
                            <span>v{version}{installSize ? ` · ${formatBytes(installSize)}` : ""}</span>
                        </>
                    )}
                </div>
                <HeroShowcaseReal/>
            </div>
        </section>
    );
}
