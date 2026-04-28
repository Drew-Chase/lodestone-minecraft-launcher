import {Ic} from "./Icons.tsx";
import Logo from "./Logo.tsx";

import instanceImg from "../img/screens/instance-page.webp";
import syncImg from "../img/screens/sync-modal.webp";
import discoverImg from "../img/screens/discover.webp";

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

export default function ShowcaseSplits() {
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
