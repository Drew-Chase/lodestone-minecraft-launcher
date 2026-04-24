import {useNavigate} from "react-router-dom";
import SourceBadge from "./SourceBadge";
import {I} from "../shell/icons";
import {cardSurfaceStyle} from "../surfaces";
import type {ContentItem} from "../../types/content";
import {formatCount, timeAgo} from "../../types/content";

const columns = "2.5fr 0.9fr 0.7fr 0.6fr 0.6fr 0.6fr 110px";
const headers = ["Name", "Author", "Source", "Downloads", "Follows", "Updated", "Loaders"];

export default function BrowseTable({items}: {items: ContentItem[]}) {
    return (
        <div className="border border-line rounded-xl overflow-hidden" style={cardSurfaceStyle}>
            <div
                style={{
                    display: "grid", gridTemplateColumns: columns,
                    padding: "10px 14px", background: "rgba(0,0,0,0.25)",
                }}
            >
                {headers.map(h => (
                    <span
                        key={h}
                        style={{
                            fontSize: 10, fontWeight: 700, color: "var(--ink-3)",
                            fontFamily: "var(--mono)", letterSpacing: 1, textTransform: "uppercase",
                        }}
                    >
                        {h}
                    </span>
                ))}
            </div>
            {items.map((item, i) => (
                <TableRow key={`${item.id}-${i}`} item={item} odd={i % 2 === 1}/>
            ))}
        </div>
    );
}

function TableRow({item, odd}: {item: ContentItem; odd: boolean}) {
    const navigate = useNavigate();
    const loaders = "loaders" in item ? (item as {loaders: string[]}).loaders : [];

    return (
        <div
            className="cursor-pointer transition-colors hover:bg-white/[0.03]"
            style={{
                display: "grid", gridTemplateColumns: columns,
                padding: "10px 14px", alignItems: "center",
                background: odd ? "rgba(255,255,255,0.015)" : "transparent",
            }}
            onClick={() => navigate(`/discover/${item.platform.toLowerCase()}/${item.slug || item.id}`)}
        >
            <div className="flex items-center gap-3">
                <div
                    className="flex-shrink-0 rounded-md overflow-hidden flex items-center justify-center"
                    style={{width: 32, height: 32, background: "rgba(0,0,0,0.3)"}}
                >
                    {item.icon_url ? (
                        <img src={item.icon_url} alt="" className="w-full h-full object-cover"/>
                    ) : (
                        <I.box size={16} style={{color: "var(--ink-4)"}}/>
                    )}
                </div>
                <span style={{fontSize: 12, fontWeight: 600}} className="truncate">{item.title}</span>
            </div>
            <span style={{fontSize: 11, color: "var(--ink-2)"}} className="truncate">
                {item.authors?.map(a => a.name).join(", ") || "—"}
            </span>
            <SourceBadge platform={item.platform}/>
            <span style={{fontSize: 11, color: "var(--ink-2)", fontFamily: "var(--mono)"}}>
                {formatCount(item.downloads)}
            </span>
            <span style={{fontSize: 11, color: "var(--ink-2)", fontFamily: "var(--mono)"}}>
                {formatCount(item.follows)}
            </span>
            <span style={{fontSize: 11, color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                {timeAgo(item.updated)}
            </span>
            <span style={{fontSize: 10, color: "var(--ink-3)"}} className="truncate">
                {loaders.join(", ") || "—"}
            </span>
        </div>
    );
}
