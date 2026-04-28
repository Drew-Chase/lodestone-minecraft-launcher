type Quote = {
    message: string;
    displayName: string;
    handle: string;
    avatar: string;
    color: string;
}
const quotes: Quote[] = [
    {message: "Switched from the official launcher and immediately my cold-start time was cut in half. Co-op Sync alone is worth it.", displayName: "pixelpoet", handle: "@pixelpoet", avatar: "P", color: "#ff5ec8"},
    {message: "I run 6 modpacks for different friend groups. Lodestone's instance system finally makes that bearable. The Modrinth + CurseForge merge is a killer feature.", displayName: "ShadowCrafter_92", handle: "@shadowcrafter", avatar: "S", color: "#22ff84"},
    {message: "Open source, no telemetry, and the UI doesn't feel like it was made in 2014. I'm not going back.", displayName: "Maple_Mori", handle: "@maple_mori", avatar: "M", color: "#ffb545"},
    {message: "The party feature is wild. We launched a 6-person Cobblemon server straight from the launcher with synced versions. Took 2 minutes total.", displayName: "dustbrick", handle: "@dustbrick", avatar: "d", color: "#47d9ff"},
    {message: "I write modpacks for a living and Lodestone is now my dev environment. JVM args, Java picker, environment vars — everything I need is in the instance settings.", displayName: "Ironhart", handle: "@ironhart_dev", avatar: "I", color: "#9747ff"},
    {message: "It's just so much faster. The launcher feels like a native app, not a webview pretending to be one.", displayName: "red_panda", handle: "@red_panda", avatar: "r", color: "#ff5a7a"}
];

export default function Quotes()
{
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
                            <div className="quote-text">{q.message}</div>
                            <div className="quote-who">
                                <div className="quote-avatar" style={{background: `linear-gradient(135deg, ${q.color}, color-mix(in oklab, ${q.color} 50%, #000))`}}>{q.avatar}</div>
                                <div>
                                    <div className="quote-name">{q.displayName}</div>
                                    <div className="quote-handle">{q.handle}</div>
                                </div>
                            </div>
                        </div>
                    ))}
                </div>
            </div>
        </section>
    );
}
