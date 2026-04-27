import {useEffect, useLayoutEffect, useRef, useState} from "react";
import {Ic} from "../components/Icons.tsx";
import Logo from "../components/Logo.tsx";
import {
    useReleaseContext,
    detectOS,
    getOSLabel,
    pickPrimaryAsset,
    formatBytes,
} from "../hooks/useRelease.ts";

import libraryImg from "../img/screens/library.webp";
import discoverImg from "../img/screens/discover.webp";
import instanceImg from "../img/screens/instance-page.webp";
import syncImg from "../img/screens/sync-modal.webp";
import newInstanceImg from "../img/screens/new-instance-modal.webp";

function Shot({src, label}: { src: string; label: string }) {
    return (
        <div className="showcase-window">
            <div className="showcase-chrome">
                <div className="dots"><span/><span/><span/></div>
                <div className="url">{label}</div>
                <div style={{width: 40}}/>
            </div>
            <div className="showcase-screen shot-screen">
                <img src={src} alt={label} loading="lazy" style={{objectFit: "cover"}}/>
            </div>
        </div>
    );
}

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

function DownloadButton({className, size}: { className?: string; size?: number }) {
    const {available, loading, downloadUrl} = useReleaseContext();
    const os = detectOS();
    const osLabel = getOSLabel(os);

    if (loading) {
        return (
            <span className={className ?? "btn btn-primary btn-lg"} style={{opacity: 0.6, pointerEvents: "none"}}>
                <Ic.download size={size ?? 15}/> Loading...
            </span>
        );
    }

    if (!available) {
        return (
            <span className={className ?? "btn btn-primary btn-lg"} style={{opacity: 0.7, cursor: "default"}}>
                <Ic.download size={size ?? 15}/> Coming Soon
            </span>
        );
    }

    return (
        <a className={className ?? "btn btn-primary btn-lg"} href={downloadUrl ?? "#download"}>
            <Ic.download size={size ?? 15}/> Download for {osLabel}
        </a>
    );
}

function Hero() {
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

function Marquee() {
    const items = [
        {l: "Modrinth", v: "52,847", u: "mods"},
        {l: "CurseForge", v: "127k", u: "projects"},
        {l: "Modpacks", v: "18,392", u: "one click"},
        {l: "Shaders", v: "1,204", u: "configs"},
        {l: "Resource Packs", v: "6,180", u: "curated"},
        {l: "Datapacks", v: "2,109", u: "discoverable"},
        {l: "Languages", v: "32", u: "translations"},
        {l: "Java versions", v: "8 → 21", u: "auto-managed"},
    ];
    const all = [...items, ...items];
    return (
        <div className="marquee">
            <div className="marquee-track">
                {all.map((it, i) => (
                    <span key={i}>
                        <Ic.box size={12} style={{color: "var(--mc-green)"}}/>
                        {it.l} · <b>{it.v}</b> {it.u}
                    </span>
                ))}
            </div>
        </div>
    );
}

function Features() {
    const features = [
        {ic: "compass", tone: "", title: "Discover everything", text: "Search across Modrinth and CurseForge with one query. Filter by version, loader, category, license — and install with a single click."},
        {ic: "box", tone: "violet", title: "Profiles, not chaos", text: "Each instance gets its own mods, configs, saves, JVM args, and Java version. Duplicate, export, or share with a link."},
        {ic: "zap", tone: "cyan", title: "Faster than the official", text: "Native Rust core. Cold launch under 800ms. Parallel downloads, smart caching, and zero-config Java provisioning out of the box."},
        {ic: "download", tone: "amber", title: "One-click imports", text: "Drop a .zip, .mrpack, or CurseForge URL — Lodestone handles unpacking, dependency resolution, and Java setup. Restore from backups in seconds."},
        {ic: "users", tone: "pink", title: "Co-op Sync", text: "Mirror a friend\u2019s instance — same mods, shaders, and version — and join their world in one click. See who\u2019s online and what they\u2019re playing."},
        {ic: "shield", tone: "", title: "Safe by design", text: "Every download is hash-verified against the source. Mod signatures and integrity checks. No telemetry. Full sandbox option."},
    ];
    return (
        <section id="features">
            <div className="container">
                <div className="section-eyebrow">FEATURES</div>
                <h2 className="section-title">Everything Mojang's launcher should be.</h2>
                <p className="section-lede">
                    A complete reimagining of the Minecraft launcher: opinionated where it should be,
                    configurable where it matters, and gorgeous in every state.
                </p>
                <div className="features-grid">
                    {features.map((f, i) => {
                        const I = Ic[f.ic as keyof typeof Ic];
                        return (
                            <div key={i} className={`feature ${f.tone}`}>
                                <div className="corner"/>
                                <div className="feature-icon"><I size={18}/></div>
                                <h3>{f.title}</h3>
                                <p>{f.text}</p>
                            </div>
                        );
                    })}
                </div>

                <div className="sources-card">
                    <div>
                        <div className="section-eyebrow" style={{marginBottom: 10}}>UNIFIED LIBRARY</div>
                        <h3 style={{fontFamily: "var(--display)", fontSize: 28, fontWeight: 700, letterSpacing: -0.8, lineHeight: 1.1, marginBottom: 12}}>
                            Modrinth and CurseForge,<br/>finally in one place.
                        </h3>
                        <p style={{fontSize: 14.5, color: "var(--ink-2)", lineHeight: 1.6}}>
                            Stop juggling two websites. Lodestone queries both APIs in parallel,
                            de-duplicates results, and shows you the source on every card.
                            Install from either with the same flow.
                        </p>
                    </div>
                    <div className="source-logos">
                        <div className="source-logo">
                            <div className="swatch modrinth"><Ic.box size={16}/></div>
                            <div>
                                <div className="name">Modrinth</div>
                                <div className="meta">52,847 projects</div>
                            </div>
                        </div>
                        <div className="source-logo">
                            <div className="swatch curse"><Ic.box size={16}/></div>
                            <div>
                                <div className="name">CurseForge</div>
                                <div className="meta">127k projects</div>
                            </div>
                        </div>
                        <div className="source-logo">
                            <div className="swatch" style={{background: "rgba(151,71,255,0.15)", color: "var(--violet)"}}><Ic.code size={16}/></div>
                            <div>
                                <div className="name">GitHub</div>
                                <div className="meta">repos & releases</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}

function StatsStrip() {
    return (
        <div className="container" style={{marginTop: 80, marginBottom: 0}}>
            <div className="stats">
                <div className="stat">
                    <div className="stat-num">2.1<small>M</small></div>
                    <div className="stat-lbl">Active players</div>
                </div>
                <div className="stat">
                    <div className="stat-num">18.4<small>k</small></div>
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

function ShowcaseSplits() {
    return (
        <section id="showcase">
            <div className="container">
                <div className="section-eyebrow">DEEP DIVE</div>
                <h2 className="section-title">Built for the way you actually play.</h2>
                <p className="section-lede">Every workflow we thought a launcher should have, finally done right.</p>

                <div className="showcase-split">
                    <div className="copy">
                        <div className="chip green" style={{marginBottom: 18}}>
                            <span className="brand-mark" style={{width: 12, height: 12}}><Logo size={12}/></span> INSTANCES
                        </div>
                        <h3>Every modpack, kept beautifully separate.</h3>
                        <p>
                            Each instance gets its own mods, configs, world saves, JVM args, and Java version — no more cross-contaminated profiles.
                            Browse mods inline, toggle them with one click, and jump straight into the world.
                        </p>
                        <ul className="checks">
                            <li><span className="ic"><Ic.check size={11}/></span>Per-instance mod manager with version pinning</li>
                            <li><span className="ic"><Ic.check size={11}/></span>Worlds, screenshots, logs, and files in one tab strip</li>
                            <li><span className="ic"><Ic.check size={11}/></span>Override Java, RAM, and JVM args per profile</li>
                            <li><span className="ic"><Ic.check size={11}/></span>Duplicate, export, or share an instance with a link</li>
                        </ul>
                    </div>
                    <div className="showcase-mock">
                        <Shot src={instanceImg} label="lodestone://instance/Survival"/>
                    </div>
                </div>

                <div className="showcase-split reverse" style={{marginTop: 80}}>
                    <div className="copy">
                        <div className="chip violet" style={{marginBottom: 18, borderColor: "rgba(151,71,255,0.4)"}}>
                            <Ic.users size={11}/> COMMUNITY
                        </div>
                        <h3>Play together, from the launcher.</h3>
                        <p>
                            Friends list, party queues, and voice rooms — without alt-tabbing to Discord.
                            See exactly what your friends are playing, ask to join, or launch the same modpack as a group.
                        </p>
                        <ul className="checks">
                            <li><span className="ic"><Ic.check size={11}/></span>Live presence — see modpack, server, playtime</li>
                            <li><span className="ic"><Ic.check size={11}/></span>One-click join the same instance</li>
                            <li><span className="ic"><Ic.check size={11}/></span>Voice rooms and party chat built in</li>
                            <li><span className="ic"><Ic.check size={11}/></span>Shared instance presets — match versions automatically</li>
                        </ul>
                    </div>
                    <div className="showcase-mock">
                        <Shot src={syncImg} label="lodestone://co-op-sync"/>
                    </div>
                </div>

                <div className="showcase-split" style={{marginTop: 80}}>
                    <div className="copy">
                        <div className="chip amber" style={{marginBottom: 18, borderColor: "rgba(255,181,69,0.4)"}}>
                            <Ic.compass size={11}/> DISCOVER
                        </div>
                        <h3>50,000 mods, one search bar.</h3>
                        <p>
                            Search across Modrinth and CurseForge in a single query. Sort by trending,
                            filter by loader and version, preview every screenshot — and install with one click,
                            fully verified and version-pinned.
                        </p>
                        <ul className="checks">
                            <li><span className="ic"><Ic.check size={11}/></span>Unified search across two platforms</li>
                            <li><span className="ic"><Ic.check size={11}/></span>Hash-verified installs, no rogue jars</li>
                            <li><span className="ic"><Ic.check size={11}/></span>Auto-resolves dependencies and Java versions</li>
                            <li><span className="ic"><Ic.check size={11}/></span>Side-by-side compare and bulk install</li>
                        </ul>
                    </div>
                    <div className="showcase-mock">
                        <Shot src={discoverImg} label="lodestone://discover"/>
                    </div>
                </div>
            </div>
        </section>
    );
}

function Steps() {
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

function Quotes() {
    const quotes = [
        {txt: "Switched from the official launcher and immediately my cold-start time was cut in half. Co-op Sync alone is worth it.", n: "pixelpoet", h: "@pixelpoet", a: "P", c: "#ff5ec8"},
        {txt: "I run 6 modpacks for different friend groups. Lodestone's instance system finally makes that bearable. The Modrinth + CurseForge merge is a killer feature.", n: "ShadowCrafter_92", h: "@shadowcrafter", a: "S", c: "#22ff84"},
        {txt: "Open source, no telemetry, and the UI doesn't feel like it was made in 2014. I'm not going back.", n: "Maple_Mori", h: "@maple_mori", a: "M", c: "#ffb545"},
        {txt: "The party feature is wild. We launched a 6-person Cobblemon server straight from the launcher with synced versions. Took 2 minutes total.", n: "dustbrick", h: "@dustbrick", a: "d", c: "#47d9ff"},
        {txt: "I write modpacks for a living and Lodestone is now my dev environment. JVM args, Java picker, environment vars — everything I need is in the instance settings.", n: "Ironhart", h: "@ironhart_dev", a: "I", c: "#9747ff"},
        {txt: "It's just so much faster. The launcher feels like a native app, not a webview pretending to be one.", n: "red_panda", h: "@red_panda", a: "r", c: "#ff5a7a"},
    ];
    return (
        <section id="community">
            <div className="container">
                <div className="section-eyebrow">COMMUNITY</div>
                <h2 className="section-title">Loved by 2 million crafters.</h2>
                <p className="section-lede">
                    From speedrunners to modpack authors to weekend builders, Lodestone is the launcher people switch to and stay on.
                </p>
                <div className="quotes-grid">
                    {quotes.map((q, i) => (
                        <div key={i} className="quote">
                            <div className="quote-text">{q.txt}</div>
                            <div className="quote-who">
                                <div className="quote-avatar" style={{background: `linear-gradient(135deg, ${q.c}, color-mix(in oklab, ${q.c} 50%, #000))`}}>{q.a}</div>
                                <div>
                                    <div className="quote-name">{q.n}</div>
                                    <div className="quote-handle">{q.h}</div>
                                </div>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </section>
    );
}

function Pricing() {
    const [yearly, setYearly] = useState(false);
    const toggleRef = useRef<HTMLDivElement>(null);
    const monthRef = useRef<HTMLButtonElement>(null);
    const yearRef = useRef<HTMLButtonElement>(null);
    const [knob, setKnob] = useState({left: 5, width: 0});
    useLayoutEffect(() => {
        const target = yearly ? yearRef.current : monthRef.current;
        if (!target || !toggleRef.current) return;
        const tRect = toggleRef.current.getBoundingClientRect();
        const r = target.getBoundingClientRect();
        setKnob({left: r.left - tRect.left, width: r.width});
    }, [yearly]);

    const tiers = [
        {
            cls: "solo", name: "Solo Sync", monthly: 3, yearly: 29,
            tag: "Sync your instances across every device you own.", cta: "Start Solo Sync",
            feats: [
                {t: "Sync up to 5 of your devices", on: true}, {t: "Unlimited personal instances", on: true},
                {t: "Mods, configs, saves, screenshots", on: true}, {t: "Per-instance world cloud snapshots", on: true},
                {t: "Resume where you left off, anywhere", on: true}, {t: "Share instances with friends", on: false},
            ],
        },
        {
            cls: "featured", name: "Friends", monthly: 6, yearly: 58,
            tag: "Run modpacks with the people you actually play with.", cta: "Get Friends Sync", badge: "MOST POPULAR",
            feats: [
                {t: "Everything in Solo Sync", on: true}, {t: "Share up to 5 instances with friends", on: true},
                {t: "Auto-match versions across the party", on: true}, {t: "Synced world saves for SMP groups", on: true},
                {t: "Live mod-update notifications", on: true}, {t: "Priority server bandwidth", on: true},
            ],
        },
        {
            cls: "unlimited", name: "Unlimited", monthly: 12, yearly: 115,
            tag: "For modpack authors, communities, and overengineers.", cta: "Go Unlimited",
            feats: [
                {t: "Everything in Friends", on: true}, {t: "Unlimited shared instances", on: true},
                {t: "Unlimited devices", on: true}, {t: "Custom CDN — push updates to thousands", on: true},
                {t: "Modpack analytics & crash reports", on: true}, {t: "Priority support · early features", on: true},
            ],
        },
    ];

    const fmtMo = (n: number) => { const v = n / 12; return v % 1 === 0 ? String(v) : v.toFixed(2); };

    return (
        <section id="pricing">
            <div className="container" style={{textAlign: "center"}}>
                <div className="section-eyebrow" style={{justifyContent: "center"}}>PRICING</div>
                <h2 className="section-title" style={{textAlign: "center"}}>The launcher is free. Sync is optional.</h2>
                <p className="section-lede" style={{margin: "0 auto", textAlign: "center"}}>
                    Every feature of Lodestone works locally, forever, at no cost.
                    Subscribe only if you want your instances to follow you between devices — or your whole friend group.
                </p>
                <div ref={toggleRef} className="billing-toggle">
                    <span className="knob" style={{left: knob.left, width: knob.width}}/>
                    <button ref={monthRef} className={!yearly ? "on" : ""} onClick={() => setYearly(false)}>Monthly</button>
                    <button ref={yearRef} className={yearly ? "on" : ""} onClick={() => setYearly(true)}>
                        Yearly <span className="save-pill">SAVE 20%</span>
                    </button>
                </div>
                <div className="pricing-grid">
                    {tiers.map((t) => {
                        const price = yearly ? fmtMo(t.yearly) : t.monthly;
                        return (
                            <div key={t.name} className={`price-card ${t.cls}`} style={{textAlign: "left"}}>
                                {"badge" in t && t.badge && <div className="badge">{t.badge}</div>}
                                <div className="price-name">{t.name}</div>
                                <div className="price-amount">
                                    <span style={{fontSize: 32, color: "var(--ink-3)", fontWeight: 600}}>$</span>
                                    {price}
                                    <span className="per">/ month</span>
                                </div>
                                <div className="price-billed">
                                    {yearly ? (<><s>${t.monthly}/mo</s><span className="save">${t.yearly} billed yearly</span></>) : (<span>billed monthly · cancel anytime</span>)}
                                </div>
                                <div className="price-tagline">{t.tag}</div>
                                <ul className="feats">
                                    {t.feats.map((f, i) => (
                                        <li key={i} className={f.on ? "" : "muted"}>
                                            <span className="ic">{f.on ? <Ic.check size={11}/> : "–"}</span>{f.t}
                                        </li>
                                    ))}
                                </ul>
                                <a className="price-cta" href="#signup"><Ic.arrowR size={13}/>{t.cta}</a>
                            </div>
                        );
                    })}
                </div>
                <div className="price-foot">7-day free trial on every tier · <a href="#sync-faq">how sync works</a></div>
            </div>
        </section>
    );
}

function OpenSource() {
    const {available, downloadUrl} = useReleaseContext();
    return (
        <section id="open-source">
            <div className="container">
                <div className="section-eyebrow">OPEN SOURCE</div>
                <h2 className="section-title">The launcher is free. Forever.</h2>
                <p className="section-lede">
                    The Lodestone launcher is MIT-licensed and developed publicly on GitHub.
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
                            <a className="btn btn-ghost btn-lg" href="https://github.com/drew-chase/lodestone-minecraft-launcher" target="_blank" rel="noopener noreferrer"><Ic.heart size={13}/> Sponsor on GitHub</a>
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
                                <div className="lbl">MIT licensed</div>
                                <div className="sub">Use, fork, redistribute freely</div>
                            </div>
                        </div>
                        <div className="oss-row">
                            <div className="oss-ic" style={{background: "rgba(71,217,255,0.15)", color: "var(--cyan)"}}><Ic.users size={16}/></div>
                            <div style={{flex: 1}}>
                                <div className="lbl">312 contributors</div>
                                <div className="sub">From 47 countries</div>
                            </div>
                        </div>
                        <div className="oss-row">
                            <div className="oss-ic" style={{background: "rgba(255,181,69,0.15)", color: "var(--amber)"}}><Ic.heart size={16}/></div>
                            <div style={{flex: 1}}>
                                <div className="lbl">2,438 sponsors</div>
                                <div className="sub">$14,200 / month covers infra</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}

function FinalCTA() {
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

function Footer() {
    return (
        <footer>
            <div className="container">
                <div className="foot-grid">
                    <div className="foot-brand">
                        <a className="brand" href="#top">
                            <span className="brand-mark"><Logo size={26}/></span>
                            Lodestone
                        </a>
                        <p>A modern, open-source Minecraft launcher built by the community, for the community. Not affiliated with Mojang or Microsoft.</p>
                    </div>
                    <div className="foot-col">
                        <h5>Product</h5>
                        <a href="#features">Features</a>
                        <a href="#download">Download</a>
                        <a href="#changelog">Changelog</a>
                        <a href="#roadmap">Roadmap</a>
                    </div>
                    <div className="foot-col">
                        <h5>Resources</h5>
                        <a href="https://github.com/drew-chase/lodestone-minecraft-launcher/wiki" target="_blank" rel="noopener noreferrer">Documentation</a>
                        <a href="#api">Modpack API</a>
                        <a href="#brand">Brand kit</a>
                        <a href="#status">Status</a>
                    </div>
                    <div className="foot-col">
                        <h5>Community</h5>
                        <a href="#discord">Discord</a>
                        <a href="https://github.com/drew-chase/lodestone-minecraft-launcher" target="_blank" rel="noopener noreferrer">GitHub</a>
                        <a href="#twitter">Twitter</a>
                        <a href="#mastodon">Mastodon</a>
                    </div>
                    <div className="foot-col">
                        <h5>Company</h5>
                        <a href="#about">About</a>
                        <a href="#sponsor">Sponsor</a>
                        <a href="#contact">Contact</a>
                        <a href="#privacy">Privacy</a>
                    </div>
                </div>
                <div className="foot-bottom">
                    <div>© 2026 Lodestone Project · MIT License · "Minecraft" is a trademark of Mojang Synergies AB.</div>
                    <div className="links">
                        <a href="#discord"><Ic.discord size={16}/></a>
                        <a href="#twitter"><Ic.twitter size={14}/></a>
                        <a href="https://github.com/drew-chase/lodestone-minecraft-launcher" target="_blank" rel="noopener noreferrer"><Ic.github size={16}/></a>
                    </div>
                </div>
            </div>
        </footer>
    );
}

export default function Home() {
    return (
        <>
            <Hero/>
            <Marquee/>
            <Features/>
            <ShowcaseSplits/>
            <Steps/>
            <Quotes/>
            <Pricing/>
            <OpenSource/>
            <FinalCTA/>
            <Footer/>
        </>
    );
}
