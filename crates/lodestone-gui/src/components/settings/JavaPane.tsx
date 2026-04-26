import {useCallback, useEffect, useState} from "react";
import {Button, Progress, Slider} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {SettingCard} from "./primitives";
import {useSettings} from "../../context/SettingsContext";
import type {DetectedJava, JavaInstallProgress, MojangRuntime} from "../../types/settings";

export default function JavaPane() {
    const {settings, update, systemRamMb} = useSettings();

    const [systemJavas, setSystemJavas] = useState<DetectedJava[]>([]);
    const [mojangRuntimes, setMojangRuntimes] = useState<MojangRuntime[]>([]);
    const [loadingJavas, setLoadingJavas] = useState(true);
    const [installingComponent, setInstallingComponent] = useState<string | null>(null);
    const [installProgress, setInstallProgress] = useState(0);

    const refreshRuntimes = useCallback(async () => {
        try {
            const [system, mojang] = await Promise.all([
                invoke<DetectedJava[]>("detect_system_java"),
                invoke<MojangRuntime[]>("get_available_java_runtimes"),
            ]);
            setSystemJavas(system);
            setMojangRuntimes(mojang);
        } catch (e) {
            console.warn("failed to detect java:", e);
        } finally {
            setLoadingJavas(false);
        }
    }, []);

    useEffect(() => {
        refreshRuntimes();
    }, [refreshRuntimes]);

    // Listen for install progress events
    useEffect(() => {
        if (!installingComponent) return;

        const unlisten = listen<JavaInstallProgress>("java-install-progress", (event) => {
            const {component, filesDownloaded, filesTotal} = event.payload;
            if (component === installingComponent && filesTotal > 0) {
                setInstallProgress(Math.round((filesDownloaded / filesTotal) * 100));
            }
        });

        return () => {
            unlisten.then((fn) => fn());
        };
    }, [installingComponent]);

    const handleInstall = async (component: string) => {
        setInstallingComponent(component);
        setInstallProgress(0);
        try {
            await invoke<string>("install_java_runtime", {component});
            await refreshRuntimes();
        } catch (e) {
            console.error("java install failed:", e);
        } finally {
            setInstallingComponent(null);
            setInstallProgress(0);
        }
    };

    const formatMemory = (mb: number): string => {
        if (mb >= 1024) {
            const gb = mb / 1024;
            return gb % 1 === 0 ? `${gb} GB` : `${gb.toFixed(1)} GB`;
        }
        return `${mb} MB`;
    };

    const recommendedMb = Math.min(Math.floor(systemRamMb / 2 / 256) * 256, systemRamMb);

    return (
        <>
            <SettingCard
                title="Memory allocation"
                desc="Maximum RAM given to a Minecraft instance."
            >
                <Slider
                    minValue={512}
                    maxValue={systemRamMb}
                    step={256}
                    value={settings.maxMemoryMb}
                    onChange={(v) => update("maxMemoryMb", v as number)}
                    aria-label="Memory allocation"
                    showTooltip
                    tooltipProps={{content: formatMemory(settings.maxMemoryMb)}}
                    classNames={{
                        track: "bg-line h-2",
                        filler: "bg-gradient-to-r from-[var(--cyan)] to-mc-green",
                        thumb: "bg-white w-[18px] h-[18px] shadow-[0_2px_6px_rgba(0,0,0,0.5),0_0_0_4px_rgba(34,255,132,0.25)]",
                    }}
                    renderValue={() => (
                        <div className="font-mono text-sm font-bold min-w-[60px] text-right">
                            {formatMemory(settings.maxMemoryMb)}
                        </div>
                    )}
                />
                <div className="flex justify-between text-[0.625rem] text-ink-3 font-mono mt-2">
                    <span>512 MB · minimum</span>
                    <span>{formatMemory(recommendedMb)} · recommended</span>
                    <span>{formatMemory(systemRamMb)} · system max</span>
                </div>
            </SettingCard>

            <SettingCard
                title="Installed runtimes"
                desc="Per-instance Java — newest versions auto-downloaded."
            >
                <div className="flex flex-col gap-2">
                    {loadingJavas ? (
                        <div className="text-xs text-ink-3 py-4 text-center">Detecting Java installations...</div>
                    ) : (
                        <>
                            {/* System-detected runtimes */}
                            {systemJavas.map((r, i) => {
                                const isDefault = settings.defaultJavaPath === r.path;
                                return (
                                    <div
                                        key={`system-${i}`}
                                        className="flex items-center gap-3 px-3.5 py-3 rounded-[10px] border"
                                        style={{
                                            borderColor: isDefault
                                                ? "rgba(34,255,132,0.4)"
                                                : "var(--line)",
                                            background: isDefault ? "rgba(34,255,132,0.05)" : "transparent",
                                        }}
                                    >
                                        <I.cpu
                                            size={15}
                                            className={isDefault ? "text-mc-green" : "text-ink-3"}
                                        />
                                        <div className="flex-1 min-w-0">
                                            <div className="text-[0.8125rem] font-semibold">{r.label}</div>
                                            <div className="text-[0.625rem] text-ink-4 font-mono overflow-hidden text-ellipsis whitespace-nowrap">
                                                {r.path}
                                            </div>
                                        </div>
                                        {isDefault ? (
                                            <Chip variant="green" className="text-[0.5625rem]">
                                                DEFAULT
                                            </Chip>
                                        ) : (
                                            <Button
                                                variant="bordered"
                                                size="sm"
                                                className="text-[0.6875rem]"
                                                onPress={() => update("defaultJavaPath", r.path)}
                                            >
                                                Use
                                            </Button>
                                        )}
                                    </div>
                                );
                            })}

                            {/* Mojang downloadable runtimes */}
                            {mojangRuntimes.map((r) => {
                                const isInstalling = installingComponent === r.component;
                                return (
                                    <div
                                        key={`mojang-${r.component}`}
                                        className="flex items-center gap-3 px-3.5 py-3 rounded-[10px] border border-line"
                                    >
                                        <I.download
                                            size={15}
                                            className={r.installed ? "text-mc-green" : "text-ink-3"}
                                        />
                                        <div className="flex-1 min-w-0">
                                            <div className="text-[0.8125rem] font-semibold">
                                                {r.label}
                                                <span className="text-ink-4 font-normal ml-1.5">· Mojang</span>
                                            </div>
                                            {isInstalling && (
                                                <Progress
                                                    size="sm"
                                                    value={installProgress}
                                                    color="success"
                                                    className="mt-1.5"
                                                    aria-label={`Installing ${r.label}`}
                                                />
                                            )}
                                        </div>
                                        {r.installed ? (
                                            <Chip variant="green" className="text-[0.5625rem]">
                                                INSTALLED
                                            </Chip>
                                        ) : isInstalling ? (
                                            <span className="text-[0.6875rem] text-ink-3 font-mono">
                                                {installProgress}%
                                            </span>
                                        ) : (
                                            <Button
                                                variant="bordered"
                                                size="sm"
                                                className="text-[0.6875rem]"
                                                onPress={() => handleInstall(r.component)}
                                                isDisabled={!!installingComponent}
                                            >
                                                Download
                                            </Button>
                                        )}
                                    </div>
                                );
                            })}
                        </>
                    )}

                    {systemJavas.length === 0 && mojangRuntimes.length === 0 && !loadingJavas && (
                        <div className="text-xs text-ink-3 py-2 text-center">
                            No Java installations found. Download one above or add a custom path.
                        </div>
                    )}
                </div>
            </SettingCard>

            <SettingCard title="JVM arguments" desc="Passed to java before -jar.">
                <textarea
                    className="w-full font-mono text-xs text-mc-green rounded-lg p-3.5 leading-relaxed whitespace-pre-wrap break-all border border-line bg-[#05060a] resize-y min-h-[80px] focus:outline-none focus:border-mc-green/40"
                    value={settings.jvmArguments}
                    onChange={(e) => update("jvmArguments", e.target.value)}
                    rows={3}
                />
            </SettingCard>
        </>
    );
}
