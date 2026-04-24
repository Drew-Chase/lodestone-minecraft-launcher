import {Button, Card} from "@heroui/react";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {cardSurfaceStyle} from "../library/instances";

type LogLevel = "INFO" | "WARN" | "ERROR";

type LogLine = {t: string; lvl: LogLevel; txt: string};

const logLines: LogLine[] = [
    {t: "18:42:03", lvl: "INFO", txt: "Loading Fabric Loader 0.15.7"},
    {t: "18:42:04", lvl: "INFO", txt: "Found 87 mods to load"},
    {t: "18:42:04", lvl: "INFO", txt: "Injecting mixin patches..."},
    {t: "18:42:05", lvl: "WARN", txt: "Mod `create` registered legacy recipe handler"},
    {t: "18:42:06", lvl: "INFO", txt: "Built registry snapshot (3,214 entries)"},
    {t: "18:42:07", lvl: "INFO", txt: "Loading world: Aether_Dimensions"},
    {t: "18:42:08", lvl: "INFO", txt: "Chunks generated: 892 · Entities: 1,204"},
    {t: "18:42:09", lvl: "INFO", txt: "Ready · 58 FPS · 2.1 GB used"},
];

const levelColor: Record<LogLevel, string> = {
    INFO: "var(--mc-green)",
    WARN: "var(--amber)",
    ERROR: "#ff5e5e",
};

// Terminal-style log viewer: file header with LIVE chip + Export button, then a
// stream of timestamped log lines, capped by a blinking-caret prompt.
export default function LogsTab() {
    return (
        <Card
            className="p-4 border border-line font-mono text-[0.71875rem] leading-[1.7]"
            style={cardSurfaceStyle}
        >
            {/* File header */}
            <div className="flex items-center gap-2.5 mb-2.5 pb-2.5 border-b border-line">
                <I.terminal size={14} className="text-mc-green"/>
                <div className="text-xs font-semibold font-sans">latest.log</div>
                <div className="flex-1"/>
                <Chip variant="green">
                    <span className="pulse-dot" style={{width: 5, height: 5}}/> LIVE
                </Chip>
                <Button
                    variant="bordered"
                    size="sm"
                    className="text-[0.6875rem]"
                    startContent={<I.download size={11}/>}
                >
                    Export
                </Button>
            </div>

            {/* Lines */}
            {logLines.map((l, i) => (
                <div key={i} className="flex gap-3">
                    <span className="text-ink-4">{l.t}</span>
                    <span className="w-[50px]" style={{color: levelColor[l.lvl]}}>
            [{l.lvl}]
          </span>
                    <span className="text-ink-1">{l.txt}</span>
                </div>
            ))}

            {/* Blinking prompt */}
            <div className="flex gap-3 items-center">
                <span className="text-ink-4">18:42:10</span>
                <span className="w-[50px] text-mc-green">[INFO]</span>
                <span className="text-ink-1">Awaiting player input</span>
                <span className="caret"/>
            </div>
        </Card>
    );
}
