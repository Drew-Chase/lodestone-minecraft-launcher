import {useNavigate} from "react-router-dom";
import {Table, TableHeader, TableColumn, TableBody, TableRow, TableCell, Spinner} from "@heroui/react";
import SourceBadge from "./SourceBadge";
import {I} from "../shell/icons";
import type {ContentItem} from "../../types/content";
import {formatCount, timeAgo} from "../../types/content";

const columns = [
    {key: "name", label: "Name", className: "w-[30%] min-w-[120px]"},
    {key: "author", label: "Author", className: "w-[14%] min-w-[80px]"},
    {key: "source", label: "Source", className: "w-[12%] min-w-[80px]"},
    {key: "downloads", label: "Downloads", className: "w-[11%] min-w-[60px]"},
    {key: "follows", label: "Follows", className: "w-[10%] min-w-[50px]"},
    {key: "updated", label: "Updated", className: "w-[11%] min-w-[55px]"},
    {key: "loaders", label: "Loaders", className: "w-[12%] min-w-[60px]"},
];

interface BrowseTableProps {
    items: ContentItem[];
    hasMore: boolean;
    loading: boolean;
    sentinelRef: React.RefCallback<HTMLElement>;
}

export default function BrowseTable({items, hasMore, loading, sentinelRef}: BrowseTableProps) {
    const navigate = useNavigate();

    return (
        <Table
            isHeaderSticky
            removeWrapper
            aria-label="Browse content"
            classNames={{
                base: "flex-1 min-h-0 overflow-y-auto border border-line rounded-xl",
                table: "w-full table-fixed",
                thead: "[&>tr]:first:rounded-none",
                th: [
                    "bg-[#0a1a10] text-[10px] font-bold text-[var(--mc-green)] font-[var(--mono)]",
                    "uppercase tracking-wider py-3 px-3",
                    "border-b border-[rgba(34,255,132,0.2)]",
                    "first:rounded-none last:rounded-none",
                ].join(" "),
                td: "text-[11px] py-2.5 px-3",
                tr: "cursor-pointer transition-colors hover:bg-white/[0.03] border-b border-line/30 last:border-b-0",
            }}
            bottomContent={
                hasMore ? (
                    <div
                        ref={sentinelRef}
                        className="flex items-center justify-center py-4"
                    >
                        {loading && <Spinner size="sm" color="success"/>}
                    </div>
                ) : items.length > 0 ? (
                    <div
                        className="flex items-center justify-center py-4"
                        style={{fontSize: 12, color: "var(--ink-4)", fontFamily: "var(--mono)"}}
                    >
                        End of results
                    </div>
                ) : null
            }
        >
            <TableHeader columns={columns}>
                {(col) => (
                    <TableColumn key={col.key} className={col.className}>
                        {col.label}
                    </TableColumn>
                )}
            </TableHeader>
            <TableBody items={items.map((item, i) => ({...item, _idx: i}))}>
                {(item) => {
                    const loaders = "loaders" in item ? (item as unknown as {loaders: string[]}).loaders : [];
                    return (
                        <TableRow
                            key={`${item.id}-${item._idx}`}
                            onClick={() => navigate(`/discover/${item.platform.toLowerCase()}/${item.slug || item.id}`)}
                        >
                            <TableCell>
                                <div className="flex items-center gap-3 min-w-0">
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
                                    <span className="truncate" style={{fontSize: 12, fontWeight: 600}}>
                                        {item.title}
                                    </span>
                                </div>
                            </TableCell>
                            <TableCell>
                                <span className="truncate block" style={{color: "var(--ink-2)"}}>
                                    {item.authors?.map(a => a.name).join(", ") || "—"}
                                </span>
                            </TableCell>
                            <TableCell>
                                <SourceBadge platform={item.platform}/>
                            </TableCell>
                            <TableCell>
                                <span style={{color: "var(--ink-2)", fontFamily: "var(--mono)"}}>
                                    {formatCount(item.downloads)}
                                </span>
                            </TableCell>
                            <TableCell>
                                <span style={{color: "var(--ink-2)", fontFamily: "var(--mono)"}}>
                                    {formatCount(item.follows)}
                                </span>
                            </TableCell>
                            <TableCell>
                                <span style={{color: "var(--ink-3)", fontFamily: "var(--mono)"}}>
                                    {timeAgo(item.updated)}
                                </span>
                            </TableCell>
                            <TableCell>
                                <span className="truncate block" style={{fontSize: 10, color: "var(--ink-3)"}}>
                                    {loaders.join(", ") || "—"}
                                </span>
                            </TableCell>
                        </TableRow>
                    );
                }}
            </TableBody>
        </Table>
    );
}
