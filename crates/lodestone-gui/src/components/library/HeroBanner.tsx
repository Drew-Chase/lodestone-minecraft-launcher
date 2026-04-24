import {Button} from "@heroui/react";
import Scene from "../shell/Scene";
import Particles from "../shell/Particles";
import Chip from "../Chip";
import {I} from "../shell/icons";
import type {Instance} from "./instances";

type HeroBannerProps = {
    featured: Instance;
};

// The "now playing" hero at the top of the Library. Scene + particles backdrop,
// chips, title, description, Resume Session cluster, and the stats pill.
export default function HeroBanner({featured}: HeroBannerProps) {
    return (
        <div className="relative h-[260px] rounded-[20px] overflow-hidden mb-7 shadow-[0_20px_40px_-20px_rgba(0,0,0,0.8)]">
            <Scene biome={featured.biome} seed={featured.seed}/>
            <Particles count={12}/>
            <div
                className="absolute inset-0 flex items-end p-8"
                style={{
                    // Horizontal fade uses three rgba stops — keep inline.
                    background:
                        "linear-gradient(90deg, rgba(8,9,10,0.92) 0%, rgba(8,9,10,0.6) 45%, transparent 80%)",
                }}
            >
                <div className="max-w-[500px]">
                    <div className="flex items-center gap-2 mb-3">
                        <Chip variant="green">
                            <span className="pulse-dot" style={{width: 6, height: 6}}/> NOW PLAYING
                        </Chip>
                        <Chip variant="violet">FABRIC 0.14.21</Chip>
                        <Chip>MC 1.20.1</Chip>
                    </div>
                    <div className="text-[2.5rem] font-extrabold -tracking-[1px] leading-none mb-2">
                        {featured.name}
                    </div>
                    <div className="text-[0.8125rem] text-ink-2 mb-5 leading-relaxed">
                        Climb the floating islands, battle Slider bosses, uncover a continent hanging in the void.
                        <span className="text-mc-green ml-1.5">{featured.mods} mods</span>{" "}
                        · last played {featured.lastPlayed}
                    </div>
                    <div className="flex gap-2.5">
                        <Button
                            color="success"
                            size="lg"
                            className="font-bold px-6"
                            startContent={<I.play size={14}/>}
                        >
                            Resume Session
                        </Button>
                        <Button
                            isIconOnly
                            variant="bordered"
                            aria-label="Open Folder"
                            className="w-[42px] h-[42px]"
                        >
                            <I.folder size={16}/>
                        </Button>
                        <Button
                            isIconOnly
                            variant="bordered"
                            aria-label="Configure"
                            className="w-[42px] h-[42px]"
                        >
                            <I.settings size={16}/>
                        </Button>
                        <Button
                            isIconOnly
                            variant="bordered"
                            aria-label="More"
                            className="w-[42px] h-[42px]"
                        >
                            <I.more size={16}/>
                        </Button>
                    </div>
                </div>
            </div>
            {/* Stats pill */}
            <div className="font-mono absolute top-6 right-6 text-[0.6875rem] text-ink-2 px-2.5 py-1.5 rounded-md border border-[rgba(255,255,255,0.08)] bg-[rgba(0,0,0,0.55)] backdrop-blur-md flex items-center gap-1.5">
                <I.cpu size={12}/> 2.1 GB / 6 GB · 58 FPS
            </div>
        </div>
    );
}
