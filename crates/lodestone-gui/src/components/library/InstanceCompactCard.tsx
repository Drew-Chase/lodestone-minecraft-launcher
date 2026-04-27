import {Button} from "@heroui/react";
import {useNavigate} from "react-router-dom";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {cardHoverClass, cardSurfaceStyle, toSlug, type Instance} from "./instances";
import InstanceActionsDropdown from "./InstanceActionsDropdown";
import {useLaunch} from "../../context/LaunchContext";

type Props = {
    instance: Instance;
    onDeleteRequest: (inst: Instance) => void;
};

// Single dense tile in the Compact view: name on row 1, MC version + loader chip
// + inline Play button on row 2. Used in a 4-up grid. Click navigates to detail.
export default function InstanceCompactCard({instance: inst, onDeleteRequest}: Props) {
    const navigate = useNavigate();
    const {launchInstance, stopInstance, isRunning, isInstalling} = useLaunch();
    const running = isRunning(inst.id);
    const installing = isInstalling(inst.id);
    const openDetail = () => navigate(`/library/${toSlug(inst.name)}`);
    const stop = (e: React.MouseEvent) => e.stopPropagation();

    return (
        // Plain div for consistency with InstanceGridCard (and to avoid HeroUI Card's
        // hover-scale effect fighting the shared cardHoverClass translate-y).
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
            className={`rounded-xl border border-line px-3.5 py-3 flex flex-col gap-2 cursor-pointer ${cardHoverClass}`}
            style={{...cardSurfaceStyle, maxWidth: 500}}
        >
            {/* Row 1 */}
            <div className="flex items-center gap-1.5 min-w-0">
                <div className="text-xs font-bold tracking-tight overflow-hidden text-ellipsis whitespace-nowrap flex-1 text-left">
                    {inst.name}
                </div>
                {running && (
                    <span
                        className="pulse-dot flex-shrink-0"
                        style={{width: 6, height: 6}}
                    />
                )}
                <div onClick={stop}>
                    <InstanceActionsDropdown instance={inst} onDeleteRequest={onDeleteRequest}>
                        <Button
                            isIconOnly
                            variant="flat"
                            size="sm"
                            aria-label="More"
                            className="w-[22px] h-[22px] min-w-0 flex-shrink-0"
                        >
                            <I.more size={12}/>
                        </Button>
                    </InstanceActionsDropdown>
                </div>
            </div>
            {/* Row 2 */}
            <div className="font-mono flex items-center gap-1.5 text-[0.625rem] text-ink-3">
                <span>{inst.mc}</span>
                <span className="opacity-50">·</span>
                <Chip variant={inst.color} className="text-[0.5625rem] px-1.5 py-px">
                    {inst.loader}
                </Chip>
                <div className="flex-1"/>
                <div onClick={stop}>
                    {running ? (
                        <Button color="danger" size="sm" className="font-bold min-w-0 h-auto px-2 py-1 text-[0.625rem]" startContent={<I.x size={9}/>} onPress={() => stopInstance(inst.id)}>
                            Stop
                        </Button>
                    ) : (
                        <Button color="success" size="sm" className="font-bold min-w-0 h-auto px-2 py-1 text-[0.625rem]" startContent={<I.play size={9}/>} isDisabled={installing} onPress={() => launchInstance(inst.id)}>
                            {installing ? "..." : "Play"}
                        </Button>
                    )}
                </div>
            </div>
        </div>
    );
}
