import {Button} from "@heroui/react";
import {open as shellOpen} from "@tauri-apps/plugin-shell";
import {I} from "../shell/icons";
import SourceBadge from "./SourceBadge";
import type {ContentItem} from "../../types/content";
import {formatCount} from "../../types/content";

export default function DetailHero({item: base}: {item: ContentItem}) {
    return (
        <div
            className="relative flex-shrink-0"
            style={{
                height: 230,
                background: "linear-gradient(180deg, rgba(8,9,10,0.3) 0%, rgba(8,9,10,0.85) 85%, var(--bg-0) 100%)",
                overflow: "hidden",
            }}
        >
            {/* Background: first gallery image or solid */}
            {base.gallery.length > 0 && (
                <img
                    src={base.gallery[0]}
                    alt=""
                    className="absolute inset-0 w-full h-full object-cover"
                    style={{opacity: 0.3, filter: "blur(8px)"}}
                />
            )}
            <div
                className="absolute inset-0"
                style={{background: "linear-gradient(180deg, rgba(8,9,10,0.3) 0%, rgba(8,9,10,0.85) 85%, var(--bg-0) 100%)"}}
            />

            {/* Content */}
            <div className="relative flex items-end gap-5 h-full" style={{padding: "0 28px 24px"}}>
                {/* Icon */}
                <div
                    className="flex-shrink-0 rounded-2xl overflow-hidden flex items-center justify-center"
                    style={{
                        width: 110,
                        height: 110,
                        background: "var(--bg-1)",
                        boxShadow: "0 20px 40px -10px rgba(0,0,0,0.8), 0 0 0 3px var(--bg-0)",
                    }}
                >
                    {base.icon_url ? (
                        <img src={base.icon_url} alt={base.title} className="w-full h-full object-cover"/>
                    ) : (
                        <I.box size={48} style={{color: "var(--ink-4)"}}/>
                    )}
                </div>

                {/* Text */}
                <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1 flex-wrap">
                        <SourceBadge platform={base.platform}/>
                        {base.categories.slice(0, 3).map(c => (
                            <span
                                key={c}
                                style={{
                                    padding: "2px 8px",
                                    borderRadius: 999,
                                    fontSize: 10,
                                    fontWeight: 600,
                                    background: "rgba(255,255,255,0.06)",
                                    color: "var(--ink-2)",
                                }}
                            >
                                {c}
                            </span>
                        ))}
                    </div>
                    <h1 style={{fontSize: 32, fontWeight: 800, letterSpacing: -0.6, margin: 0}}>
                        {base.title}
                    </h1>
                    <div className="flex items-center gap-4 mt-1" style={{fontSize: 12, color: "var(--ink-3)"}}>
                        <span>{base.authors.map(a => a.name).join(", ") || "Unknown author"}</span>
                        <span className="flex items-center gap-1" style={{fontFamily: "var(--mono)"}}>
                            <I.download size={12}/> {formatCount(base.downloads)}
                        </span>
                        <span className="flex items-center gap-1" style={{fontFamily: "var(--mono)"}}>
                            <I.heart size={12}/> {formatCount(base.follows)}
                        </span>
                    </div>
                </div>

                {/* Actions */}
                <div className="flex gap-2">
                    {base.links.website && (
                        <Button
                            variant="bordered"
                            size="sm"
                            className="border-line text-ink-2"
                            onPress={() => shellOpen(base.links.website!)}
                            startContent={<I.external size={14}/>}
                        >
                            Website
                        </Button>
                    )}
                    <Button
                        className="bg-mc-green text-[#072010] font-bold"
                        size="sm"
                        startContent={<I.download size={14}/>}
                    >
                        Install
                    </Button>
                </div>
            </div>
        </div>
    );
}
