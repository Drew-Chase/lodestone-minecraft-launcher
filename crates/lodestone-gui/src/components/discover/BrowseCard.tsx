import {Button} from "@heroui/react";
import {cardSurfaceStyle, cardHoverClass} from "../surfaces";
import SourceBadge from "./SourceBadge";
import {I} from "../shell/icons";
import type {ContentItem} from "../../types/content";
import {formatCount, timeAgo} from "../../types/content";

interface BrowseCardProps {
    item: ContentItem;
    onClick?: () => void;
    onInstall?: () => void;
    installing?: boolean;
    installed?: boolean;
}

export default function BrowseCard({item, onClick, onInstall, installing, installed}: BrowseCardProps) {
    const loaders = "loaders" in item ? (item as {loaders: string[]}).loaders : [];

    return (
        <div
            className={`border border-line rounded-xl overflow-hidden cursor-pointer ${cardHoverClass}`}
            style={cardSurfaceStyle}
            onClick={onClick}
        >
            {/* Image / icon area */}
            <div
                className="relative flex items-center justify-center"
                style={{height: 130, background: "rgba(0,0,0,0.25)", overflow: "hidden"}}
            >
                {item.icon_url ? (
                    <img src={item.icon_url} alt={item.title} className="w-full h-full object-cover"/>
                ) : (
                    <I.box size={40} style={{color: "var(--ink-4)"}}/>
                )}

                {item.categories.length > 0 && (
                    <div className="absolute top-2 left-2 flex gap-1 flex-wrap">
                        {item.categories.slice(0, 2).map(c => (
                            <span
                                key={c}
                                style={{
                                    fontSize: 9, padding: "2px 6px", borderRadius: 4,
                                    background: "rgba(0,0,0,0.5)", color: "var(--ink-2)",
                                    backdropFilter: "blur(6px)",
                                }}
                            >
                                {c}
                            </span>
                        ))}
                    </div>
                )}

                <div className="absolute bottom-2 left-2">
                    <SourceBadge platform={item.platform}/>
                </div>
            </div>

            {/* Text content */}
            <div style={{padding: "12px 14px"}}>
                <div style={{fontSize: 13, fontWeight: 700, marginBottom: 2}} className="truncate">
                    {item.title}
                </div>
                <div style={{fontSize: 10, color: "var(--ink-3)"}} className="truncate">
                    {item.authors?.map(a => a.name).join(", ") || "Unknown author"}
                </div>
                <div
                    style={{
                        fontSize: 11, color: "var(--ink-2)", lineHeight: 1.45,
                        minHeight: 32, marginTop: 4,
                        display: "-webkit-box", WebkitLineClamp: 2,
                        WebkitBoxOrient: "vertical", overflow: "hidden",
                    }}
                >
                    {item.summary}
                </div>
                <div className="flex items-center gap-2 mt-2" style={{fontSize: 11, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                    <span className="flex items-center gap-1">
                        <I.download size={11}/> {formatCount(item.downloads)}
                    </span>
                    <span className="flex items-center gap-1">
                        <I.heart size={11}/> {formatCount(item.follows)}
                    </span>
                    {loaders.length > 0 && (
                        <span className="flex items-center gap-1">
                            <I.cpu size={11}/> {loaders.slice(0, 2).join(", ")}
                        </span>
                    )}
                    {!onInstall && <span className="ml-auto">{timeAgo(item.updated)}</span>}
                    {onInstall && (
                        <div className="ml-auto" onClick={e => e.stopPropagation()}>
                            <InstallBtn installing={installing} installed={installed} onInstall={onInstall}/>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}

/** Shared install button used in grid, compact, and list views. */
export function InstallBtn({installing, installed, onInstall}: {installing?: boolean; installed?: boolean; onInstall: () => void}) {
    if (installed) return <Button size="sm" variant="flat" className="font-bold text-mc-green" isDisabled startContent={<I.check size={12}/>}>Installed</Button>;
    if (installing) return <Button size="sm" variant="bordered" isDisabled startContent={<div className="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"/>}>Installing</Button>;
    return <Button color="success" size="sm" className="font-bold" startContent={<I.download size={12}/>} onPress={onInstall}>Install</Button>;
}
