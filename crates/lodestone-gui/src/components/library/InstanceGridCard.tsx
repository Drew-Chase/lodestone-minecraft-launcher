import {Button} from "@heroui/react";
import {useNavigate} from "react-router-dom";
import Scene from "../shell/Scene";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {cardHoverClass, cardSurfaceStyle, toSlug, type Instance} from "./instances";
import InstanceActionsDropdown from "./InstanceActionsDropdown";
import {useLaunch} from "../../context/LaunchContext";
import {useInstanceImage} from "../../hooks/useInstanceImage";

type Props = {
    instance: Instance;
    onDeleteRequest: (inst: Instance) => void;
};

export default function InstanceGridCard({instance: inst, onDeleteRequest}: Props) {
    const navigate = useNavigate();
    const {launchInstance, stopInstance, isRunning, isInstalling} = useLaunch();
    const running = isRunning(inst.id);
    const installing = isInstalling(inst.id);
    const bannerUrl = useInstanceImage(inst.instancePath, "banner.png");
    const iconUrl = useInstanceImage(inst.instancePath, "icon.png");
    const openDetail = () => navigate(`/library/${toSlug(inst.name)}`);
    const stop = (e: React.MouseEvent) => e.stopPropagation();

    return (
        <div
            role="button"
            tabIndex={0}
            onClick={openDetail}
            onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    openDetail();
                }
            }}
            className={`rounded-xl overflow-hidden border border-line cursor-pointer ${cardHoverClass}`}
            style={{...cardSurfaceStyle, maxWidth: 500}}
        >
            {/* Thumbnail */}
            <div className="relative w-full" style={{height: 130}}>
                {bannerUrl ? (
                    <img src={bannerUrl} alt="" className="absolute inset-0 w-full h-full object-cover"/>
                ) : (
                    <Scene biome={inst.biome} seed={inst.seed}/>
                )}
                {running && (
                    <div className="absolute top-2 left-2 z-[2]">
                        <Chip variant="green" className="text-[0.5625rem] px-1.5 py-0.5">
                            <span className="pulse-dot" style={{width: 5, height: 5}}/>
                            PLAYING
                        </Chip>
                    </div>
                )}
                <div className="absolute top-2 right-2 z-[2]" onClick={stop}>
                    <InstanceActionsDropdown instance={inst} onDeleteRequest={onDeleteRequest}>
                        <Button
                            isIconOnly
                            size="sm"
                            variant="flat"
                            aria-label="More"
                            className="w-7 h-7 min-w-0 bg-[rgba(0,0,0,0.5)] backdrop-blur-sm"
                        >
                            <I.more size={14}/>
                        </Button>
                    </InstanceActionsDropdown>
                </div>
                <div
                    className="absolute inset-0"
                    style={{
                        background:
                            "linear-gradient(180deg, transparent 50%, rgba(0,0,0,0.75) 100%)",
                    }}
                />
                <div className="absolute bottom-2.5 left-3 right-3 flex items-end z-[2] gap-2.5">
                    {iconUrl && (
                        <div className="w-10 h-10 rounded-lg flex-shrink-0 overflow-hidden" style={{boxShadow: "0 4px 12px rgba(0,0,0,0.5)"}}>
                            <img src={iconUrl} alt="" className="w-full h-full object-cover"/>
                        </div>
                    )}
                    <div className="min-w-0 flex-1">
                        <div className="text-[0.8125rem] font-bold truncate">{inst.name}</div>
                        <div className="font-mono text-[0.625rem] text-ink-2">{inst.version}</div>
                    </div>
                    <div onClick={stop}>
                        {running ? (
                            <Button color="danger" size="sm" className="font-bold" startContent={<I.x size={11}/>} onPress={() => stopInstance(inst.id)}>
                                Stop
                            </Button>
                        ) : (
                            <Button color="success" size="sm" className="font-bold" startContent={<I.play size={11}/>} isDisabled={installing} onPress={() => launchInstance(inst.id)}>
                                {installing ? "..." : "Play"}
                            </Button>
                        )}
                    </div>
                </div>
            </div>

            {/* Stats footer */}
            <div style={{padding: "10px 14px"}} className="flex items-center gap-2.5 text-[0.6875rem] text-ink-3">
                <span className="flex items-center gap-1">
                    <I.clock size={11}/> {inst.playtime}
                </span>
                <span>·</span>
                <span className="flex items-center gap-1">
                    <I.box size={11}/> {inst.mods} mods
                </span>
                <div className="flex-1"/>
                <Chip variant={inst.color} className="text-[0.5625rem] px-1.5 py-0.5">
                    {inst.loader}
                </Chip>
            </div>
        </div>
    );
}
