export default function Quotes() {
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
