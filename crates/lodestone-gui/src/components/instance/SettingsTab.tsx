import {Button} from "@heroui/react";
import {I} from "../shell/icons";
import {
    SelectSettingRow,
    SettingCard,
    ToggleRow,
} from "../settings/primitives";
import type {Instance} from "../library/instances";

type Props = {
    instance: Instance;
};

// Per-instance overrides. Mirrors the global Settings primitives (SettingCard /
// ToggleRow / SelectSettingRow) but scopes each control to just this instance.
// Controls are display-only for now — the global Settings screen is already
// wired the same way and persistence is out of scope across both places.
export default function SettingsTab({instance}: Props) {
    return (
        <>
            {/* Memory override */}
            <SettingCard
                title="Memory allocation"
                desc={`Override the global default for ${instance.name}.`}
            >
                <div className="flex items-center gap-3.5 mb-4">
                    <div className="relative flex-1 h-2 bg-line rounded">
                        <div
                            className="absolute left-0 w-[60%] h-full rounded"
                            style={{
                                background: "linear-gradient(90deg, var(--cyan), var(--mc-green))",
                                boxShadow: "0 0 12px var(--mc-green-glow)",
                            }}
                        />
                        <div
                            className="absolute w-[18px] h-[18px] rounded-full bg-white -top-[5px]"
                            style={{
                                left: "60%",
                                transform: "translateX(-50%)",
                                boxShadow:
                                    "0 2px 6px rgba(0,0,0,0.5), 0 0 0 4px rgba(34,255,132,0.25)",
                            }}
                        />
                    </div>
                    <div className="font-mono text-sm font-bold min-w-[60px] text-right">
                        10 GB
                    </div>
                </div>
                <div className="flex justify-between text-[0.625rem] text-ink-3 font-mono">
                    <span>2 GB · minimum</span>
                    <span>10 GB · this instance</span>
                    <span>16 GB · system max</span>
                </div>
            </SettingCard>

            {/* Runtime + misc launch toggles */}
            <SettingCard>
                <div className="-m-1">
                    <SelectSettingRow
                        label="Java runtime"
                        value={`Java 21 (bundled) · matches ${instance.mc}`}
                    />
                    <SelectSettingRow label="Window resolution" value="1920 × 1080"/>
                    <ToggleRow
                        label="Launch fullscreen"
                        desc="Start Minecraft maximized on the active monitor"
                    />
                    <ToggleRow
                        label="Show log window on launch"
                        desc="Open the Logs tab automatically when the game starts"
                        on
                    />
                    <ToggleRow
                        label="Open to LAN by default"
                        desc="Allow friends on the same network to join without a port forward"
                        last
                    />
                </div>
            </SettingCard>

            {/* JVM args */}
            <SettingCard
                title="JVM arguments"
                desc="Passed to java before -jar. Overrides the global defaults."
            >
                <div
                    className="font-mono text-xs text-mc-green rounded-lg p-3.5 leading-relaxed whitespace-pre-wrap break-all border border-line"
                    style={{background: "#05060a"}}
                >
                    {"-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200\n-XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch"}
                </div>
            </SettingCard>

            {/* Instance management */}
            <SettingCard
                title="Instance management"
                desc="Duplicate, export, or remove this instance."
            >
                <div className="flex gap-2.5 flex-wrap">
                    <Button
                        variant="bordered"
                        size="sm"
                        startContent={<I.copy size={13}/>}
                    >
                        Duplicate instance
                    </Button>
                    <Button
                        variant="bordered"
                        size="sm"
                        startContent={<I.upload size={13}/>}
                    >
                        Export as .mrpack
                    </Button>
                    <Button
                        variant="bordered"
                        size="sm"
                        className="text-[#ff6b6b] border-[rgba(255,107,107,0.3)]"
                        startContent={<I.trash size={13}/>}
                    >
                        Delete instance
                    </Button>
                </div>
            </SettingCard>
        </>
    );
}
