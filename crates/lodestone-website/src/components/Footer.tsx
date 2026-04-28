import {Ic} from "./Icons.tsx";
import Logo from "./Logo.tsx";

export default function Footer() {
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
