import PageShell from "../../components/auth/PageShell.tsx";

interface Color {
    name: string;
    hex: string;
    use: string;
}

// The palette from base.css. The hex values are the ones the site renders.
const COLORS: Color[] = [
    {name: "Lodestone green", hex: "#22ff84", use: "The logo, buttons and links. Our main accent."},
    {name: "Green, dim", hex: "#18c968", use: "Hover and pressed states on green."},
    {name: "Green, deep", hex: "#0a5c33", use: "Shadows and glows behind green elements."},
    {name: "Background", hex: "#08090a", use: "The page background."},
    {name: "Surface", hex: "#0e1012", use: "Panels and cards."},
    {name: "White", hex: "#ffffff", use: "Headings and primary text."},
    {name: "Violet", hex: "#9747ff", use: "A secondary accent for labels."},
    {name: "Amber", hex: "#ffb545", use: "Notices and warnings."},
    {name: "Cyan", hex: "#47d9ff", use: "Information."},
];

interface Face {
    name: string;
    role: string;
    sample: string;
    fontFamily?: string;
}

// The three typefaces. Inter is the body font. Space Grotesk and JetBrains Mono are loaded from Google Fonts.
const FACES: Face[] = [
    {name: "Inter", role: "Body text, labels and buttons", sample: "Sync your instances across every device you own."},
    {name: "Space Grotesk", role: "Headings", sample: "Your instances, everywhere", fontFamily: "var(--display)"},
    {name: "JetBrains Mono", role: "Versions and code", sample: "1.21.4 · Fabric", fontFamily: "var(--mono)"},
];

export default function BrandPage() {
    return (
        <PageShell wide>
            <article className="brand-page">
                <header className="brand-intro">
                    <h1 className="acct-title">Brand kit</h1>
                    <p className="acct-body">
                        The logo, colors and type for writing about Lodestone or linking to it. Use the assets as they are.
                    </p>
                </header>

                <section className="brand-section" aria-labelledby="brand-logo">
                    <h2 id="brand-logo" className="brand-h2">Logo</h2>
                    <div className="brand-logo-tile">
                        <img src="/lodestone-logo.svg" alt="The Lodestone logo" width={128} height={128}/>
                    </div>
                    <div className="brand-downloads">
                        <a className="btn btn-primary" href="/lodestone-logo.svg" download>Download SVG</a>
                        <a className="btn btn-ghost" href="/android-chrome-512x512.png" download>Download PNG (512 px)</a>
                    </div>
                    <ul className="brand-rules">
                        <li className="acct-body">Keep the logo's proportions. Don't stretch, rotate, recolor or add effects to it.</li>
                        <li className="acct-body">Leave clear space around it, and don't place it over busy images.</li>
                        <li className="acct-body">Don't use the logo in a way that suggests Mojang, Microsoft or we endorse your project.</li>
                    </ul>
                </section>

                <section className="brand-section" aria-labelledby="brand-name">
                    <h2 id="brand-name" className="brand-h2">Name</h2>
                    <p className="acct-body">
                        Write the name as Lodestone. When you need to say what it is, write "Lodestone, a Minecraft launcher."
                        Don't use the name for a product of your own.
                    </p>
                </section>

                <section className="brand-section" aria-labelledby="brand-color">
                    <h2 id="brand-color" className="brand-h2">Colors</h2>
                    <ul className="brand-swatches">
                        {COLORS.map((color) => (
                            <li key={color.hex} className="brand-swatch">
                                <span className="brand-swatch-chip" style={{background: color.hex}} aria-hidden="true"/>
                                <span className="brand-swatch-name">{color.name}</span>
                                <span className="brand-swatch-hex">{color.hex}</span>
                                <span className="brand-swatch-use">{color.use}</span>
                            </li>
                        ))}
                    </ul>
                </section>

                <section className="brand-section" aria-labelledby="brand-type">
                    <h2 id="brand-type" className="brand-h2">Type</h2>
                    <ul className="brand-type">
                        {FACES.map((face) => (
                            <li key={face.name} className="brand-type-row">
                                <span className="brand-type-name">{face.name} · {face.role}</span>
                                <p className="brand-type-sample" style={face.fontFamily ? {fontFamily: face.fontFamily} : undefined}>
                                    {face.sample}
                                </p>
                            </li>
                        ))}
                    </ul>
                </section>

                <section className="brand-section" aria-labelledby="brand-trademarks">
                    <h2 id="brand-trademarks" className="brand-h2">Trademarks</h2>
                    <p className="acct-body">
                        Lodestone is not affiliated with Mojang or Microsoft. "Minecraft" is a trademark of Mojang Synergies AB.
                        Follow the <a href="https://www.minecraft.net/en-us/usage-guidelines" target="_blank" rel="noopener noreferrer">Minecraft usage guidelines</a> when you mention it.
                    </p>
                </section>

                <section className="brand-section" aria-labelledby="brand-contact">
                    <h2 id="brand-contact" className="brand-h2">Contact</h2>
                    <p className="acct-body">
                        For press, partnerships or brand questions, email <a href="mailto:contact@lodestonelauncher.com">contact@lodestonelauncher.com</a>.
                    </p>
                </section>
            </article>
        </PageShell>
    );
}
