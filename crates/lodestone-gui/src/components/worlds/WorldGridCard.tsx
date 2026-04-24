import {Button} from "@heroui/react";
import Scene from "../shell/Scene";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {cardHoverClass, cardSurfaceStyle} from "../surfaces";
import WorldMode from "./WorldMode";
import type {World} from "./worldsData";

type Props = {
    world: World;
};

// Single world tile in the grid view. Scene thumbnail with PINNED / BACKED UP
// chips overlaid, gradient fade at the bottom, name + instance/version mono
// line, then a stat row (WorldMode + playtime + size), seed pill, and action
// cluster (copy seed, more, Load).
export default function WorldGridCard({world: w}: Props) {
    return (
        <div
            className={`rounded-lg overflow-hidden border border-line cursor-pointer ${cardHoverClass}`}
            style={cardSurfaceStyle}
        >
            <div className="relative h-[150px]">
                <Scene biome={w.biome} seed={w.seed}/>

                {/* Top chips — pinned left, backed-up right */}
                <div className="absolute top-2.5 left-2.5 right-2.5 flex gap-1.5 z-[2]">
                    {w.pinned && (
                        <Chip variant="green" className="text-[0.5625rem] px-[7px] py-0.5">
                            <I.pin size={9}/> PINNED
                        </Chip>
                    )}
                    <div className="flex-1"/>
                    {w.backed && (
                        <span
                            className="font-mono font-bold tracking-[0.5px] flex items-center gap-1 text-accent-cyan rounded-full"
                            style={{
                                fontSize: "0.5625rem",
                                padding: "3px 7px",
                                background: "rgba(71,217,255,0.14)",
                                border: "1px solid rgba(71,217,255,0.3)",
                            }}
                        >
              <I.shield size={9}/> BACKED UP
            </span>
                    )}
                </div>

                {/* Fade + title overlay */}
                <div
                    className="absolute inset-0"
                    style={{
                        background:
                            "linear-gradient(180deg, transparent 50%, rgba(0,0,0,0.8) 100%)",
                    }}
                />
                <div className="absolute bottom-2.5 left-3 right-3 z-[2]">
                    <div className="text-[0.9375rem] font-extrabold -tracking-[0.3px] mb-0.5">
                        {w.name}
                    </div>
                    <div className="text-[0.625rem] text-ink-2 font-mono">
                        {w.inst} · {w.version}
                    </div>
                </div>
            </div>

            {/* Stat row */}
            <div className="px-3.5 py-2.5 flex items-center gap-2.5 text-[0.6875rem] text-ink-3 font-mono">
                <WorldMode gamemode={w.gamemode} difficulty={w.difficulty}/>
                <div className="flex-1"/>
                <span className="flex items-center gap-0.5" title="Playtime">
          <I.clock size={10}/> {w.played}
        </span>
                <span className="flex items-center gap-0.5" title="Size">
          <I.hardDrive size={10}/> {w.size}
        </span>
            </div>

            {/* Seed + actions */}
            <div className="px-3.5 pb-3 flex items-center gap-1.5">
                <div className="flex-1 min-w-0 px-2 py-1.5 rounded-md border border-line bg-[rgba(255,255,255,0.03)] text-[0.625rem] text-ink-3 font-mono overflow-hidden text-ellipsis whitespace-nowrap">
                    SEED · {w.seedStr}
                </div>
                <Button
                    isIconOnly
                    variant="bordered"
                    size="sm"
                    aria-label="Copy seed"
                    className="w-[30px] h-[30px] min-w-0"
                >
                    <I.copy size={13}/>
                </Button>
                <Button
                    isIconOnly
                    variant="bordered"
                    size="sm"
                    aria-label="More"
                    className="w-[30px] h-[30px] min-w-0"
                >
                    <I.more size={13}/>
                </Button>
                <Button
                    color="success"
                    size="sm"
                    className="font-bold text-[0.6875rem]"
                    startContent={<I.play size={11}/>}
                >
                    Load
                </Button>
            </div>
        </div>
    );
}
