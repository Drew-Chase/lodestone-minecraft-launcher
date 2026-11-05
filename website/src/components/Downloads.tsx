import {useRef, useState} from "react";
import {Ic} from "./Icons.tsx";
import {useReleaseContext, pickPrimaryAsset, detectOS} from "../hooks/useRelease.ts";

const INSTALL_COMMAND = "curl -fsSL https://lodestonelauncher.com/install | sh";

// The platform downloads, with the Linux one-line installer beneath them. The section is the `#download` target the
// footer links to.
export default function Downloads() {
    const {available, loading, version, publishedAt, windows, macos, linux} = useReleaseContext();
    const [copied, setCopied] = useState(false);
    const commandRef = useRef<HTMLElement>(null);

    const fmtDate = (iso: string | null) => {
        if (!iso) return null;
        return new Date(iso).toLocaleDateString("en-US", {month: "long", day: "numeric", year: "numeric"});
    };

    // The visitor's own OS gets the primary style.
    const current = detectOS();
    const platforms = [
        {os: "macos", label: "macOS", meta: "14+ · ARM64 + Intel", icon: Ic.apple, asset: pickPrimaryAsset(macos)},
        {os: "windows", label: "Windows", meta: "10+ · x64", icon: Ic.win, asset: pickPrimaryAsset(windows)},
        {os: "linux", label: "Linux", meta: "x86_64 · tar.gz", icon: Ic.linux, asset: pickPrimaryAsset(linux)},
    ];

    const copyCommand = async () => {
        try {
            await navigator.clipboard.writeText(INSTALL_COMMAND);
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1800);
        } catch {
            // The clipboard refused the write, so select the command for the reader to copy with Ctrl+C.
            const code = commandRef.current;
            if (!code) return;
            const range = document.createRange();
            range.selectNodeContents(code);
            const selection = window.getSelection();
            selection?.removeAllRanges();
            selection?.addRange(range);
        }
    };

    return (
        <div className="downloads" id="download">
            <div className="os-buttons">
                {platforms.map(({os, label, meta, icon: Icon, asset}) => {
                    const tone = os === current ? " is-primary" : "";
                    const content = (
                        <>
                            <Icon size={16}/>
                            <span><b>{label}</b><span className="os-meta"> · {meta}</span></span>
                        </>
                    );
                    return available && asset
                        ? <a key={label} className={`os-btn${tone}`} href={asset.download_url}>{content}</a>
                        : <span key={label} className={`os-btn is-disabled${tone}`}>{content}</span>;
                })}
            </div>

            <div className="install-cmd">
                <span className="install-label">Linux, one line:</span>
                <div className="cmd">
                    <span className="cmd-prompt">$</span>
                    <code ref={commandRef}>{INSTALL_COMMAND}</code>
                    <button type="button" className={`cmd-copy${copied ? " is-done" : ""}`} onClick={copyCommand}>
                        {copied ? <Ic.check size={14}/> : <Ic.copy size={14}/>}
                        {copied ? "Copied" : "Copy"}
                    </button>
                </div>
            </div>

            {available && (
                <div className="downloads-meta">
                    v{version}{publishedAt ? ` · Released ${fmtDate(publishedAt)}` : ""} · <a href={`https://github.com/drew-chase/lodestone-minecraft-launcher/releases/tag/${version}`} target="_blank" rel="noopener noreferrer">changelog</a>
                </div>
            )}
            {!available && !loading && (
                <div className="downloads-meta">No releases yet. Check back soon, or star the repo to get notified.</div>
            )}
        </div>
    );
}
