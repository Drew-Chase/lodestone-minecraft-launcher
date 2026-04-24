import {Button, Card} from "@heroui/react";
import Scene from "../shell/Scene";
import {I} from "../shell/icons";
import {cardSurfaceStyle} from "../surfaces";
import WorldMode from "./WorldMode";
import type {World} from "./worldsData";

type Props = {
    list: World[];
};

// 7-column dense row view. Each row: 44px thumbnail, name + instance with
// optional pin icon, WorldMode indicator, seed + version mono block,
// playtime/size, last played, actions cluster (More icon + primary Load).
// Zebra striping via i % 2 matches the design.
export default function WorldTable({list}: Props) {
    return (
        <Card
            className="p-0 overflow-hidden border border-line"
            style={cardSurfaceStyle}
        >
            {list.map((w, i) => {
                const isLast = i === list.length - 1;
                return (
                    <div
                        key={i}
                        className={`grid items-center gap-3.5 px-3.5 py-2.5 text-xs cursor-pointer transition-colors hover:bg-[rgba(34,255,132,0.04)] ${
                            isLast ? "" : "border-b border-line"
                        } ${i % 2 ? "bg-[rgba(255,255,255,0.015)]" : "bg-transparent"}`}
                        style={{
                            // 7-column fractional grid — matches design's template exactly.
                            gridTemplateColumns: "54px 1fr 1.1fr 1fr 0.8fr 0.6fr 100px",
                        }}
                    >
                        <div className="w-11 h-11 rounded-md overflow-hidden relative">
                            <Scene biome={w.biome} seed={w.seed}/>
                        </div>

                        <div className="min-w-0">
                            <div className="flex items-center gap-1.5 mb-0.5">
                                {w.pinned && <I.pin size={10} className="text-mc-green"/>}
                                <span className="font-bold overflow-hidden text-ellipsis whitespace-nowrap">
                  {w.name}
                </span>
                            </div>
                            <div className="text-[0.625rem] text-ink-3 font-mono">{w.inst}</div>
                        </div>

                        <WorldMode gamemode={w.gamemode} difficulty={w.difficulty}/>

                        <div className="text-[0.625rem] text-ink-3 font-mono">
                            <div>SEED · {w.seedStr}</div>
                            <div className="mt-0.5">{w.version}</div>
                        </div>

                        <div className="text-[0.6875rem] text-ink-2 font-mono">
                            {w.played} · {w.size}
                        </div>

                        <div className="text-[0.6875rem] text-ink-3">{w.last}</div>

                        <div className="flex gap-1 justify-end">
                            <Button
                                isIconOnly
                                variant="bordered"
                                size="sm"
                                aria-label="More"
                                className="w-7 h-7 min-w-0"
                            >
                                <I.more size={12}/>
                            </Button>
                            <Button
                                color="success"
                                size="sm"
                                className="font-bold text-[0.6875rem]"
                                startContent={<I.play size={10}/>}
                            >
                                Load
                            </Button>
                        </div>
                    </div>
                );
            })}
        </Card>
    );
}
