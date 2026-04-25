import {useRef, useState, useEffect} from "react";
import {useNavigate} from "react-router-dom";
import {Tooltip} from "@heroui/react";
import {cardSurfaceStyle, cardHoverClass} from "../surfaces";
import SourceBadge from "./SourceBadge";
import {I} from "../shell/icons";
import type {ContentItem} from "../../types/content";
import {formatCount} from "../../types/content";

export default function BrowseCompact({items}: {items: ContentItem[]}) {
    return (
        <div style={{display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))", gap: 10}}>
            {items.map((item, i) => (
                <CompactCard key={`${item.id}-${i}`} item={item}/>
            ))}
        </div>
    );
}

function CompactCard({item}: {item: ContentItem}) {
    const navigate = useNavigate();
    const loaders = "loaders" in item ? (item as {loaders: string[]}).loaders : [];

    return (
        <div
            className={`border border-line rounded-xl overflow-hidden cursor-pointer flex ${cardHoverClass}`}
            style={{...cardSurfaceStyle, padding: 0}}
            onClick={() => navigate(`/discover/${item.platform.toLowerCase()}/${item.slug || item.id}`)}
        >
            <div
                className="flex-shrink-0 flex items-center justify-center"
                style={{width: 64, background: "rgba(0,0,0,0.25)"}}
            >
                {item.icon_url ? (
                    <img src={item.icon_url} alt={item.title} className="w-full h-full object-cover"/>
                ) : (
                    <I.box size={22} style={{color: "var(--ink-4)"}}/>
                )}
            </div>

            <div style={{flex: 1, padding: "6px 10px", minWidth: 0}}>
                <div className="flex items-center gap-1.5">
                    <TruncatedTitle title={item.title}/>
                    <SourceBadge platform={item.platform}/>
                </div>
                <div style={{fontSize: 9, color: "var(--ink-3)"}} className="truncate">
                    {item.authors?.map(a => a.name).join(", ") || "Unknown"}
                </div>
                <div
                    style={{
                        fontSize: 10, color: "var(--ink-2)", lineHeight: 1.4,
                        marginTop: 1, display: "-webkit-box", WebkitLineClamp: 2,
                        WebkitBoxOrient: "vertical", overflow: "hidden",
                    }}
                >
                    {item.summary}
                </div>
                <div className="flex gap-2 mt-0.5" style={{fontSize: 9, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                    <span className="flex items-center gap-1"><I.download size={9}/> {formatCount(item.downloads)}</span>
                    {loaders.length > 0 && <span>{loaders[0]}</span>}
                </div>
            </div>
        </div>
    );
}

function TruncatedTitle({title}: {title: string}) {
    const ref = useRef<HTMLSpanElement>(null);
    const [isTruncated, setIsTruncated] = useState(false);

    useEffect(() => {
        const el = ref.current;
        if (el) setIsTruncated(el.scrollWidth > el.clientWidth);
    }, [title]);

    const span = (
        <span
            ref={ref}
            style={{fontSize: 12, fontWeight: 700}}
            className="truncate flex-1 min-w-0"
        >
            {title}
        </span>
    );

    if (!isTruncated) return span;

    return (
        <Tooltip
            content={title}
            delay={400}
            classNames={{
                content: "bg-[#0d1117] border border-line text-xs text-[var(--ink-1)] max-w-[300px]",
            }}
        >
            {span}
        </Tooltip>
    );
}
