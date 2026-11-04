import {Ic} from "./Icons.tsx";

export default function Features() {
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
