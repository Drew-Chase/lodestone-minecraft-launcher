import {Button} from "@heroui/react";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {cardHoverClass, cardSurfaceStyle, type Instance} from "./instances";

type Props = {
    instance: Instance;
};

// Single dense tile in the Compact view: name on row 1, MC version + loader chip
// + inline Play button on row 2. Used in a 4-up grid.
export default function InstanceCompactCard({instance: inst}: Props) {
    return (
        // Plain div for consistency with InstanceGridCard (and to avoid HeroUI Card's
        // hover-scale effect fighting the shared cardHoverClass translate-y).
        <div
            className={`rounded-lg border border-line px-3.5 py-3 flex flex-col gap-2 cursor-pointer ${cardHoverClass}`}
            style={cardSurfaceStyle}
        >
            {/* Row 1 */}
            <div className="flex items-center gap-1.5 min-w-0">
                <div className="text-xs font-bold tracking-tight overflow-hidden text-ellipsis whitespace-nowrap flex-1 text-left">
                    {inst.name}
                </div>
                {inst.playing && (
                    <span
                        className="pulse-dot flex-shrink-0"
                        style={{width: 6, height: 6}}
                    />
                )}
                <Button
                    isIconOnly
                    variant="flat"
                    size="sm"
                    aria-label="More"
                    className="w-[22px] h-[22px] min-w-0 flex-shrink-0"
                >
                    <I.more size={12}/>
                </Button>
            </div>
            {/* Row 2 */}
            <div className="font-mono flex items-center gap-1.5 text-[0.625rem] text-ink-3">
                <span>{inst.mc}</span>
                <span className="opacity-50">·</span>
                <Chip variant={inst.color} className="text-[0.5625rem] px-1.5 py-px">
                    {inst.loader}
                </Chip>
                <div className="flex-1"/>
                <Button
                    color="success"
                    size="sm"
                    className="font-bold min-w-0 h-auto px-2 py-1 text-[0.625rem]"
                    startContent={<I.play size={9}/>}
                >
                    Play
                </Button>
            </div>
        </div>
    );
}
