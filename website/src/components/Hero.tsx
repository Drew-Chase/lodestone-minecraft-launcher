import Downloads from "./Downloads.tsx";

// The launcher's v0.7.0 demo on YouTube. The nocookie host keeps YouTube from setting cookies until someone presses play.
const DEMO_VIDEO_ID = "NZ8RtBGYpKU";

function HeroVideo() {
    return (
        <div className="hero-showcase">
            <div className="showcase-window">
                <div className="showcase-chrome">
                    <div className="dots"><span/><span/><span/></div>
                    <div className="url">lodestone://demo</div>
                    <div style={{width: 40}}/>
                </div>
                <div className="hero-video">
                    <iframe
                        src={`https://www.youtube-nocookie.com/embed/${DEMO_VIDEO_ID}?rel=0&modestbranding=1`}
                        title="Lodestone v0.7.0 demo"
                        loading="lazy"
                        referrerPolicy="strict-origin-when-cross-origin"
                        allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share"
                        allowFullScreen
                    />
                </div>
            </div>
        </div>
    );
}

export default function Hero() {
    return (
        <section className="hero" id="top">
            <div className="hero-grid"/>
            <div className="hero-aurora"/>
            <div className="container hero-inner">
                <h1>
                    The Minecraft launcher<br/>
                    you've been <span className="accent">crafting toward.</span>
                </h1>
                <p className="lede">
                    Lodestone is a fast, beautiful, open-source launcher for Java Edition.
                    Browse 50,000+ mods from Modrinth and CurseForge, manage instances,
                    import any modpack, and sync instances with friends — all in one place.
                </p>
                <Downloads/>
                <HeroVideo/>
            </div>
        </section>
    );
}
