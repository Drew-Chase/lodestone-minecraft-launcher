import {Button} from "@heroui/react";
import {useNavigate} from "react-router-dom";
import Scene from "../shell/Scene";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {cardHoverClass, cardSurfaceStyle, toSlug, type Instance} from "./instances";

type Props = {
    instance: Instance;
};

export default function InstanceGridCard({instance: inst}: Props) {
    const navigate = useNavigate();
    const openDetail = () => navigate(`/library/${toSlug(inst.name)}`);
    const stop = (e: React.MouseEvent) => e.stopPropagation();

    return (
        <div
            role="button"
            tabIndex={0}
            onClick={openDetail}
            onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    openDetail();
                }
            }}
            className={`rounded-xl overflow-hidden border border-line cursor-pointer ${cardHoverClass}`}
            style={{...cardSurfaceStyle, maxWidth: 500}}
        >
            {/* Thumbnail */}
            <div className="relative w-full" style={{height: 130}}>
                <Scene biome={inst.biome} seed={inst.seed}/>
                {inst.playing && (
                    <div className="absolute top-2 left-2 z-[2]">
                        <Chip variant="green" className="text-[0.5625rem] px-1.5 py-0.5">
                            <span className="pulse-dot" style={{width: 5, height: 5}}/>
                            PLAYING
                        </Chip>
                    </div>
                )}
                <div className="absolute top-2 right-2 z-[2]" onClick={stop}>
                    <Button
                        isIconOnly
                        size="sm"
                        variant="flat"
                        aria-label="More"
                        className="w-7 h-7 min-w-0 bg-[rgba(0,0,0,0.5)] backdrop-blur-sm"
                    >
                        <I.more size={14}/>
                    </Button>
                </div>
                <div
                    className="absolute inset-0"
                    style={{
                        background:
                            "linear-gradient(180deg, transparent 50%, rgba(0,0,0,0.75) 100%)",
                    }}
                />
                <div className="absolute bottom-2.5 left-3 right-3 flex items-end z-[2]">
                    <div className="min-w-0 flex-1">
                        <div className="text-[0.8125rem] font-bold truncate">{inst.name}</div>
                        <div className="font-mono text-[0.625rem] text-ink-2">{inst.version}</div>
                    </div>
                    <div onClick={stop}>
                        <Button
                            color="success"
                            size="sm"
                            className="font-bold"
                            startContent={<I.play size={11}/>}
                        >
                            Play
                        </Button>
                    </div>
                </div>
            </div>

            {/* Stats footer */}
            <div style={{padding: "10px 14px"}} className="flex items-center gap-2.5 text-[0.6875rem] text-ink-3">
                <span className="flex items-center gap-1">
                    <I.clock size={11}/> {inst.playtime}
                </span>
                <span>·</span>
                <span className="flex items-center gap-1">
                    <I.box size={11}/> {inst.mods} mods
                </span>
                <div className="flex-1"/>
                <Chip variant={inst.color} className="text-[0.5625rem] px-1.5 py-0.5">
                    {inst.loader}
                </Chip>
            </div>
        </div>
    );
}
