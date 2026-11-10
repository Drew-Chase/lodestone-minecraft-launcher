import {Ic} from "./Icons.tsx";
import Logo from "./Logo.tsx";

export default function Footer() {
    return (
        <footer>
            <div className="container">
                <div className="foot-grid">
                    <div className="foot-brand">
                        <a className="brand" href="/">
                            <span className="brand-mark"><Logo size={26}/></span>
                            Lodestone
                        </a>
                        <p>A modern, open-source Minecraft launcher built by the community, for the community. Not affiliated with Mojang or Microsoft.</p>
                    </div>
                    <div className="foot-col">
                        <h5>Product</h5>
                        <a href="#features">Features</a>
                        <a href="#download">Download</a>
                        <a href="#pricing">Pricing</a>
                    </div>
                    <div className="foot-col">
                        <h5>Resources</h5>
                        <a href="https://github.com/drew-chase/lodestone-minecraft-launcher#readme" target="_blank" rel="noopener noreferrer">Documentation</a>
                        <a href="/brand">Brand kit</a>
                    </div>
                    <div className="foot-col">
                        <h5>Community</h5>
                        <a href="https://github.com/drew-chase/lodestone-minecraft-launcher" target="_blank" rel="noopener noreferrer">GitHub</a>
                        <a href="https://github.com/sponsors/Drew-Chase" target="_blank" rel="noopener noreferrer">Sponsor</a>
                    </div>
                    <div className="foot-col">
                        <h5>Legal</h5>
                        <a href="/terms">Terms of Service</a>
                        <a href="/privacy">Privacy</a>
                    </div>
                </div>
                <div className="foot-bottom">
                    <div>© 2026 Lodestone Project · <a href="https://github.com/drew-chase/lodestone-minecraft-launcher/blob/master/LICENSE" target="_blank" rel="noopener noreferrer">MIT License</a> · "Minecraft" is a trademark of Mojang Synergies AB.</div>
                    <div className="links">
                        <a href="https://github.com/drew-chase/lodestone-minecraft-launcher" target="_blank" rel="noopener noreferrer" aria-label="GitHub"><Ic.github size={16}/></a>
                    </div>
                </div>
            </div>
        </footer>
    );
}
