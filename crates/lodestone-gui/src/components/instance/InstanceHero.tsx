import {Button} from "@heroui/react";
import Scene from "../shell/Scene";
import Particles from "../shell/Particles";
import Chip from "../Chip";
import {I} from "../shell/icons";
import type {Instance} from "../library/instances";

type Props = {
    instance: Instance;
    onBack?: () => void;
};

// The top-of-page hero for the Instance Detail screen. Scene background + violet
// particles, top-row action buttons, and the bottom row with the large avatar
// thumbnail (which intentionally protrudes 40px into the tab area below),
// title/chips/breadcrumb, and Pause / Running buttons.
export default function InstanceHero({instance, onBack}: Props) {
    return (
        <div className="relative h-[230px] flex-shrink-0">
            <Scene biome={instance.biome} seed={instance.seed}/>
            <Particles count={18} color="var(--violet)"/>
            <div
                className="absolute inset-0"
                style={{
                    background:
                        "linear-gradient(180deg, rgba(8,9,10,0.3) 0%, rgba(8,9,10,0.85) 85%, var(--bg-0) 100%)",
                }}
            />

            {/* Top row — back + (heart / external / more) */}
            <div className="absolute top-4 left-6 right-6 flex gap-2.5">
                <button
                    type="button"
                    onClick={onBack}
                    aria-label="Back"
                    className="w-9 h-9 rounded-md border border-line text-ink-2 flex items-center justify-center cursor-pointer backdrop-blur-md bg-[rgba(0,0,0,0.4)] hover:bg-[rgba(0,0,0,0.55)]"
                >
                    <I.chevRight size={14} className="rotate-180"/>
                </button>
                <div className="flex-1"/>
                <button
                    type="button"
                    aria-label="Favourite"
                    className="w-9 h-9 rounded-md border border-line text-ink-2 flex items-center justify-center cursor-pointer backdrop-blur-md bg-[rgba(0,0,0,0.4)] hover:bg-[rgba(0,0,0,0.55)]"
                >
                    <I.heart size={14}/>
                </button>
                <button
                    type="button"
                    aria-label="Open externally"
                    className="w-9 h-9 rounded-md border border-line text-ink-2 flex items-center justify-center cursor-pointer backdrop-blur-md bg-[rgba(0,0,0,0.4)] hover:bg-[rgba(0,0,0,0.55)]"
                >
                    <I.external size={14}/>
                </button>
                <button
                    type="button"
                    aria-label="More"
                    className="w-9 h-9 rounded-md border border-line text-ink-2 flex items-center justify-center cursor-pointer backdrop-blur-md bg-[rgba(0,0,0,0.4)] hover:bg-[rgba(0,0,0,0.55)]"
                >
                    <I.more size={14}/>
                </button>
            </div>

            {/* Bottom row — avatar + title + action cluster. Avatar is translated
                down so it visually protrudes into the content area below. */}
            <div className="absolute left-7 right-7 bottom-0 flex items-end gap-5">
                <div
                    className="w-[110px] h-[110px] rounded-[18px] flex-shrink-0 overflow-hidden relative translate-y-10"
                    style={{
                        boxShadow:
                            "0 20px 40px -10px rgba(0,0,0,0.8), 0 0 0 3px var(--bg-0)",
                    }}
                >
                    <Scene biome={instance.biome} seed={instance.seed}/>
                </div>
                <div className="pb-[18px] flex-1 min-w-0">
                    <div className="flex gap-1.5 mb-2">
                        <Chip variant="green">
                            <span className="pulse-dot" style={{width: 5, height: 5}}/> LAUNCHED
                        </Chip>
                        <Chip variant="violet">FABRIC 0.14.21</Chip>
                        <Chip>MC {instance.mc}</Chip>
                        <Chip variant="amber">{instance.mods} MODS</Chip>
                    </div>
                    <div className="text-[2rem] font-extrabold -tracking-[0.6px] leading-none">
                        {instance.name}
                    </div>
                    <div className="text-xs text-ink-3 mt-1.5 font-mono">
                        /instances/{instance.name.toLowerCase().replace(/\s+/g, "-")} · 4.8 GB · last saved 02:14
                    </div>
                </div>
                <div className="pb-[18px] flex gap-2">
                    <Button variant="bordered" size="sm" startContent={<I.pause size={13}/>}>
                        Pause
                    </Button>
                    <Button
                        color="success"
                        size="sm"
                        className="font-bold px-[22px]"
                        startContent={<I.play size={12}/>}
                    >
                        Running · 12m
                    </Button>
                </div>
            </div>
        </div>
    );
}
