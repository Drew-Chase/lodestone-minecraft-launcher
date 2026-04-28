import {Button} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import Scene from "../shell/Scene";
import Particles from "../shell/Particles";
import Chip from "../Chip";
import {I} from "../shell/icons";
import type {Instance} from "../library/instances";
import {useLaunch} from "../../context/LaunchContext";
import {useInstanceImage} from "../../hooks/useInstanceImage";

type Props = {
    instance: Instance;
    onBack?: () => void;
};

export default function InstanceHero({instance, onBack}: Props) {
    const {launchInstance, stopInstance, isRunning, isInstalling, installingInstances} = useLaunch();
    const running = isRunning(instance.id);
    const installing = isInstalling(instance.id);
    const progress = installingInstances.get(instance.id);

    const iconUrl = useInstanceImage(instance.instancePath, "icon.png");
    const bannerUrl = useInstanceImage(instance.instancePath, "banner.png");

    const loaderChip =
        instance.loader !== "Vanilla" && instance.loaderVersion
            ? `${instance.loader.toUpperCase()} ${instance.loaderVersion}`
            : instance.loader.toUpperCase();

    return (
        <div className="relative h-[230px] flex-shrink-0">
            {bannerUrl ? (
                <img
                    src={bannerUrl}
                    alt=""
                    className="absolute inset-0 w-full h-full object-cover"
                    style={{opacity: 0.5}}
                />
            ) : (
                <Scene biome={instance.biome} seed={instance.seed}/>
            )}
            <Particles count={18} color="var(--violet)"/>
            <div
                className="absolute inset-0"
                style={{
                    background:
                        "linear-gradient(180deg, rgba(8,9,10,0.3) 0%, rgba(8,9,10,0.85) 85%, var(--bg-0) 100%)",
                }}
            />

            {/* Top row */}
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
                    aria-label="Open folder"
                    onClick={() => invoke("open_directory", {path: instance.instancePath})}
                    className="w-9 h-9 rounded-md border border-line text-ink-2 flex items-center justify-center cursor-pointer backdrop-blur-md bg-[rgba(0,0,0,0.4)] hover:bg-[rgba(0,0,0,0.55)]"
                >
                    <I.folder size={14}/>
                </button>
                <button
                    type="button"
                    aria-label="More"
                    className="w-9 h-9 rounded-md border border-line text-ink-2 flex items-center justify-center cursor-pointer backdrop-blur-md bg-[rgba(0,0,0,0.4)] hover:bg-[rgba(0,0,0,0.55)]"
                >
                    <I.more size={14}/>
                </button>
            </div>

            {/* Bottom row */}
            <div className="absolute left-7 right-7 bottom-0 flex items-end gap-5">
                <div
                    className="w-[110px] h-[110px] rounded-[18px] flex-shrink-0 overflow-hidden relative translate-y-10"
                    style={{
                        boxShadow:
                            "0 20px 40px -10px rgba(0,0,0,0.8), 0 0 0 3px var(--bg-0)",
                    }}
                >
                    {iconUrl ? (
                        <img src={iconUrl} alt={instance.name} className="w-full h-full object-cover"/>
                    ) : (
                        <Scene biome={instance.biome} seed={instance.seed}/>
                    )}
                </div>
                <div className="pb-[18px] flex-1 min-w-0">
                    <div className="flex gap-1.5 mb-2 flex-wrap">
                        {running && (
                            <Chip variant="green">
                                <span className="pulse-dot" style={{width: 5, height: 5}}/> RUNNING
                            </Chip>
                        )}
                        {installing && (
                            <Chip variant="cyan">
                                <span className="pulse-dot" style={{width: 5, height: 5}}/> INSTALLING
                            </Chip>
                        )}
                        <Chip variant={instance.color}>{loaderChip}</Chip>
                        <Chip>MC {instance.mc}</Chip>
                    </div>
                    <div className="text-[2rem] font-extrabold -tracking-[0.6px] leading-none">
                        {instance.name}
                    </div>
                    <div className="text-xs text-ink-3 mt-1.5 font-mono truncate">
                        {installing && progress
                            ? progress.stageLabel
                            : `${instance.instancePath} · last played ${instance.lastPlayed}`}
                    </div>
                </div>
                <div className="pb-[18px] flex gap-2">
                    {running ? (
                        <Button
                            color="danger"
                            size="sm"
                            className="font-bold px-[22px]"
                            startContent={<I.x size={12}/>}
                            onPress={() => stopInstance(instance.id)}
                        >
                            Stop
                        </Button>
                    ) : installing ? (
                        <Button
                            color="warning"
                            size="sm"
                            className="font-bold px-[22px]"
                            isDisabled
                            startContent={
                                <div className="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"/>
                            }
                        >
                            {Math.round((progress?.progress ?? 0) * 100)}%
                        </Button>
                    ) : (
                        <Button
                            color="success"
                            size="sm"
                            className="font-bold px-[22px]"
                            startContent={<I.play size={12}/>}
                            onPress={() => launchInstance(instance.id)}
                        >
                            Play
                        </Button>
                    )}
                </div>
            </div>
        </div>
    );
}
