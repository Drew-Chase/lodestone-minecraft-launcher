import {useState, useEffect} from "react";
import Logo from "./Logo.tsx";
import {Ic} from "./Icons.tsx";
import {useReleaseContext} from "../hooks/useRelease.ts";

export default function Navigation() {
    const [open, setOpen] = useState(false);
    const {available, downloadUrl} = useReleaseContext();

    useEffect(() => {
        if (open) {
            document.body.style.overflow = "hidden";
        } else {
            document.body.style.overflow = "";
        }
        return () => { document.body.style.overflow = ""; };
    }, [open]);

    const dlHref = available && downloadUrl ? downloadUrl : "#download";

    return (
        <>
            <nav className="nav">
                <div className="container nav-inner">
                    <a className="brand" href="#top">
                        <span className="brand-mark"><Logo size={26}/></span>
                        Lodestone
                    </a>
                    <div className="nav-links">
                        <a href="#features">Features</a>
                        <a href="#pricing">Pricing</a>
                        {/*<a href="#community">Community</a>*/}
                        <a href="#open-source">Open Source</a>
                        <a href="https://github.com/drew-chase/lodestone-minecraft-launcher/wiki" target="_blank" rel="noopener noreferrer">Docs</a>
                    </div>
                    <div className="nav-cta">
                        <a className="btn btn-ghost nav-github-btn" href="https://github.com/drew-chase/lodestone-minecraft-launcher" target="_blank" rel="noopener noreferrer">
                            <Ic.github size={14}/> <span className="nav-github-text">GitHub</span>
                        </a>
                        <a className="btn btn-primary nav-download-btn" href={dlHref}>
                            <Ic.download size={14}/> <span className="nav-download-text">{available ? "Download" : "Coming Soon"}</span>
                        </a>
                        <button className="nav-burger" onClick={() => setOpen(true)} aria-label="Open menu">
                            <Ic.menu size={22}/>
                        </button>
                    </div>
                </div>
            </nav>

            {/* Mobile drawer */}
            <div className={`mobile-overlay ${open ? "open" : ""}`} onClick={() => setOpen(false)}/>
            <div className={`mobile-drawer ${open ? "open" : ""}`}>
                <div className="mobile-drawer-header">
                    <a className="brand" href="#top" onClick={() => setOpen(false)}>
                        <span className="brand-mark"><Logo size={24}/></span>
                        Lodestone
                    </a>
                    <button className="nav-burger" onClick={() => setOpen(false)} aria-label="Close menu">
                        <Ic.x size={22}/>
                    </button>
                </div>
                <div className="mobile-drawer-links">
                    <a href="#features" onClick={() => setOpen(false)}>Features</a>
                    <a href="#pricing" onClick={() => setOpen(false)}>Pricing</a>
                    {/*<a href="#community" onClick={() => setOpen(false)}>Community</a>*/}
                    <a href="#open-source" onClick={() => setOpen(false)}>Open Source</a>
                    <a href="https://github.com/drew-chase/lodestone-minecraft-launcher/wiki" target="_blank" rel="noopener noreferrer" onClick={() => setOpen(false)}>Docs</a>
                </div>
                <div className="mobile-drawer-cta">
                    <a className="btn btn-ghost" href="https://github.com/drew-chase/lodestone-minecraft-launcher" target="_blank" rel="noopener noreferrer" onClick={() => setOpen(false)} style={{width: "100%", justifyContent: "center"}}>
                        <Ic.github size={14}/> GitHub
                    </a>
                    <a className="btn btn-primary" href={dlHref} onClick={() => setOpen(false)} style={{width: "100%", justifyContent: "center"}}>
                        <Ic.download size={14}/> {available ? "Download" : "Coming Soon"}
                    </a>
                </div>
            </div>
        </>
    );
}
