import {Card} from "@heroui/react";
import Scene from "../shell/Scene";
import {I} from "../shell/icons";
import type {Biome} from "../shell/Scene";
import {cardSurfaceStyle} from "../library/instances";

const screenshots: Biome[] = [
    "end",
    "cherry",
    "nether",
    "ocean",
    "forest",
    "desert",
    "snow",
    "mushroom",
    "end",
];

// Screenshots tab — 3-column grid of thumbnails with a mono filename + size
// footer beneath each Scene render.
export default function ScreenshotsTab() {
    return (
        <div className="grid grid-cols-3 gap-3">
            {screenshots.map((b, i) => (
                <Card
                    key={i}
                    className="p-0 overflow-hidden border border-line"
                    style={cardSurfaceStyle}
                >
                    <div className="h-[160px] relative">
                        <Scene biome={b} seed={i * 11 + 7}/>
                    </div>
                    <div className="px-3 py-2.5 text-[0.6875rem] text-ink-3 font-mono flex items-center gap-2">
                        <I.image size={11}/>
                        <span>
              2026-04-{10 + i}_18.0{i}.png
            </span>
                        <div className="flex-1"/>
                        <span>2.1 MB</span>
                    </div>
                </Card>
            ))}
        </div>
    );
}
