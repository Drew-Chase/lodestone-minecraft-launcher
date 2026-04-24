import {I} from "../shell/icons";
import {cardSurfaceStyle} from "../surfaces";
import type {ContentItem} from "../../types/content";

export default function DetailSidebar({item: base, loaders}: {item: ContentItem; loaders: string[]}) {
    return (
        <div className="flex flex-col gap-4">
            {/* Links */}
            <SidebarCard title="Links">
                {base.links.website && <LinkRow icon="globe" label="Website" href={base.links.website}/>}
                {base.links.source && <LinkRow icon="external" label="Source" href={base.links.source}/>}
                {base.links.issues && <LinkRow icon="tag" label="Issues" href={base.links.issues}/>}
                {base.links.wiki && <LinkRow icon="external" label="Wiki" href={base.links.wiki}/>}
                {base.links.discord && <LinkRow icon="message" label="Discord" href={base.links.discord}/>}
            </SidebarCard>

            {/* Game versions */}
            {base.game_versions.length > 0 && (
                <SidebarCard title="Minecraft Versions">
                    <div className="flex flex-wrap gap-1">
                        {base.game_versions.slice(0, 12).map(v => (
                            <span
                                key={v}
                                style={{
                                    padding: "3px 8px",
                                    borderRadius: 6,
                                    fontSize: 10,
                                    fontFamily: "var(--mono)",
                                    fontWeight: 600,
                                    background: "rgba(255,255,255,0.04)",
                                    color: "var(--ink-2)",
                                    border: "1px solid var(--line)",
                                }}
                            >
                                {v}
                            </span>
                        ))}
                        {base.game_versions.length > 12 && (
                            <span style={{fontSize: 10, color: "var(--ink-3)", alignSelf: "center"}}>
                                +{base.game_versions.length - 12} more
                            </span>
                        )}
                    </div>
                </SidebarCard>
            )}

            {/* Loaders */}
            {loaders.length > 0 && (
                <SidebarCard title="Loaders">
                    <div className="flex flex-wrap gap-1">
                        {loaders.map(l => (
                            <span
                                key={l}
                                style={{
                                    padding: "3px 8px",
                                    borderRadius: 6,
                                    fontSize: 10,
                                    fontFamily: "var(--mono)",
                                    fontWeight: 600,
                                    background: "rgba(34,255,132,0.08)",
                                    color: "var(--mc-green)",
                                    border: "1px solid rgba(34,255,132,0.2)",
                                }}
                            >
                                {l}
                            </span>
                        ))}
                    </div>
                </SidebarCard>
            )}

            {/* License */}
            {base.license && (
                <SidebarCard title="License">
                    <span style={{fontSize: 12, color: "var(--ink-2)"}}>
                        {base.license.name || base.license.id}
                    </span>
                </SidebarCard>
            )}

            {/* Authors */}
            {base.authors.length > 0 && (
                <SidebarCard title="Authors">
                    {base.authors.map((a, i) => (
                        <div key={i} className="flex items-center gap-2" style={{marginBottom: 4}}>
                            <div
                                className="rounded-full flex items-center justify-center"
                                style={{width: 24, height: 24, background: "var(--bg-2)"}}
                            >
                                <I.user size={12} style={{color: "var(--ink-3)"}}/>
                            </div>
                            <span style={{fontSize: 12, color: "var(--ink-1)"}}>{a.name}</span>
                        </div>
                    ))}
                </SidebarCard>
            )}
        </div>
    );
}

function SidebarCard({title, children}: {title: string; children: React.ReactNode}) {
    return (
        <div className="border border-line rounded-xl" style={{...cardSurfaceStyle, padding: 16}}>
            <div
                style={{
                    fontSize: 10,
                    fontWeight: 700,
                    color: "var(--ink-3)",
                    fontFamily: "var(--mono)",
                    letterSpacing: 1,
                    textTransform: "uppercase",
                    marginBottom: 10,
                }}
            >
                {title}
            </div>
            {children}
        </div>
    );
}

function LinkRow({icon, label, href}: {icon: keyof typeof I; label: string; href: string}) {
    const IconComp = I[icon];
    return (
        <a
            href={href}
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 py-1 transition-colors hover:text-mc-green"
            style={{fontSize: 12, color: "var(--ink-2)", textDecoration: "none"}}
        >
            <IconComp size={13}/>
            <span>{label}</span>
            <I.external size={10} style={{marginLeft: "auto", opacity: 0.5}}/>
        </a>
    );
}
