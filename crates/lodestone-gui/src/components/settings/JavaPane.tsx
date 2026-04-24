import {Button} from "@heroui/react";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {SettingCard} from "./primitives";

type Runtime = {
    label: string;
    path: string;
    active?: boolean;
};

const runtimes: Runtime[] = [
    {label: "Java 21 · Temurin", path: "/Library/Java/JavaVirtualMachines/temurin-21", active: true},
    {label: "Java 17 · Zulu", path: "~/.lodestone/java/zulu-17"},
    {label: "Java 8 · Corretto", path: "~/.lodestone/java/corretto-8"},
];

export default function JavaPane() {
    return (
        <>
            <SettingCard
                title="Memory allocation"
                desc="Maximum RAM given to a Minecraft instance."
            >
                <div className="flex items-center gap-3.5 mb-4">
                    <div className="relative flex-1 h-2 bg-line rounded">
                        <div
                            className="absolute left-0 w-1/2 h-full rounded"
                            style={{
                                background: "linear-gradient(90deg, var(--cyan), var(--mc-green))",
                                boxShadow: "0 0 12px var(--mc-green-glow)",
                            }}
                        />
                        <div
                            className="absolute w-[18px] h-[18px] rounded-full bg-white -top-[5px]"
                            style={{
                                left: "50%",
                                transform: "translateX(-50%)",
                                boxShadow:
                                    "0 2px 6px rgba(0,0,0,0.5), 0 0 0 4px rgba(34,255,132,0.25)",
                            }}
                        />
                    </div>
                    <div className="font-mono text-sm font-bold min-w-[60px] text-right">
                        8 GB
                    </div>
                </div>
                <div className="flex justify-between text-[0.625rem] text-ink-3 font-mono">
                    <span>2 GB · minimum</span>
                    <span>8 GB · recommended</span>
                    <span>16 GB · system max</span>
                </div>
            </SettingCard>

            <SettingCard
                title="Installed runtimes"
                desc="Per-instance Java — newest versions auto-downloaded."
            >
                <div className="flex flex-col gap-2">
                    {runtimes.map((r, i) => (
                        <div
                            key={i}
                            className="flex items-center gap-3 px-3.5 py-3 rounded-[10px] border"
                            style={{
                                borderColor: r.active
                                    ? "rgba(34,255,132,0.4)"
                                    : "var(--line)",
                                background: r.active ? "rgba(34,255,132,0.05)" : "transparent",
                            }}
                        >
                            <I.cpu
                                size={15}
                                className={r.active ? "text-mc-green" : "text-ink-3"}
                            />
                            <div className="flex-1 min-w-0">
                                <div className="text-[0.8125rem] font-semibold">{r.label}</div>
                                <div className="text-[0.625rem] text-ink-4 font-mono overflow-hidden text-ellipsis whitespace-nowrap">
                                    {r.path}
                                </div>
                            </div>
                            {r.active ? (
                                <Chip variant="green" className="text-[0.5625rem]">
                                    DEFAULT
                                </Chip>
                            ) : (
                                <Button
                                    variant="bordered"
                                    size="sm"
                                    className="text-[0.6875rem]"
                                >
                                    Use
                                </Button>
                            )}
                        </div>
                    ))}
                    <Button
                        variant="bordered"
                        size="md"
                        className="mt-1 justify-center"
                        startContent={<I.plus size={13}/>}
                    >
                        Add custom runtime
                    </Button>
                </div>
            </SettingCard>

            <SettingCard title="JVM arguments" desc="Passed to java before -jar.">
                <div
                    className="font-mono text-xs text-mc-green rounded-lg p-3.5 leading-relaxed whitespace-pre-wrap break-all border border-line"
                    style={{background: "#05060a"}}
                >
                    {"-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200\n-XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch"}
                </div>
            </SettingCard>
        </>
    );
}
