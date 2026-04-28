import {Ic} from "./Icons.tsx";

export default function Marquee() {
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
