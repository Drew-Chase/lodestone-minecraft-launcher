import {Button, Card} from "@heroui/react";
import {useNavigate} from "react-router-dom";
import Scene from "../shell/Scene";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {cardSurfaceStyle, toSlug, type Instance} from "./instances";

type Props = {
    list: Instance[];
};

const thClass =
    "text-left px-3.5 py-2.5 text-[0.625rem] font-semibold tracking-[0.06em] uppercase text-ink-3 border-b border-line";
const tdClass =
    "px-3.5 py-3 text-xs text-ink-1 align-middle border-b border-[color-mix(in_oklab,var(--line)_60%,transparent)]";

// Dense tabular view with thumbnail column, instance, loader chip, MC version,
// mods count, playtime, last-played date, and per-row actions. Clicking a row
// navigates to /library/:slug; the actions column stops propagation so Play /
// More don't trigger the row navigation.
export default function InstanceTable({list}: Props) {
    const navigate = useNavigate();
    const stop = (e: React.MouseEvent) => e.stopPropagation();

    return (
        <Card
            className="p-0 overflow-hidden border border-line"
            style={cardSurfaceStyle}
        >
            <table className="w-full border-collapse">
                <thead>
                <tr>
                    <th className={`${thClass} w-9`}></th>
                    <th className={thClass}>Instance</th>
                    <th className={thClass}>Loader</th>
                    <th className={thClass}>MC Version</th>
                    <th className={`${thClass} text-right`}>Mods</th>
                    <th className={`${thClass} text-right`}>Playtime</th>
                    <th className={thClass}>Last Played</th>
                    <th className={`${thClass} w-[120px]`}></th>
                </tr>
                </thead>
                <tbody>
                {list.map((inst, i) => (
                    <tr
                        key={i}
                        className="cursor-pointer transition-colors hover:bg-[rgba(34,255,132,0.04)]"
                        onClick={() => navigate(`/library/${toSlug(inst.name)}`)}
                    >
                        <td className={`${tdClass} pr-0`}>
                            <div className="w-6 h-6 rounded-md overflow-hidden relative shadow-[inset_0_0_0_1px_rgba(255,255,255,0.08)]">
                                <Scene biome={inst.biome} seed={inst.seed}/>
                            </div>
                        </td>
                        <td className={tdClass}>
                            <div className="flex items-center gap-2">
                                <span className="font-semibold">{inst.name}</span>
                                {inst.playing && (
                                    <Chip variant="green" className="text-[0.5625rem] px-1.5 py-px">
                                        <span className="pulse-dot" style={{width: 4, height: 4}}/>
                                        playing
                                    </Chip>
                                )}
                            </div>
                        </td>
                        <td className={tdClass}>
                            <Chip variant={inst.color} className="text-[0.5625rem] px-[7px] py-0.5">
                                {inst.loader}
                            </Chip>
                        </td>
                        <td className={`${tdClass} font-mono text-ink-2 text-[0.6875rem]`}>{inst.mc}</td>
                        <td
                            className={`${tdClass} text-right font-mono text-[0.6875rem] ${inst.mods ? "text-ink-1" : "text-ink-3"}`}
                        >
                            {inst.mods || "—"}
                        </td>
                        <td className={`${tdClass} text-right font-mono text-[0.6875rem]`}>{inst.playtime}</td>
                        <td className={`${tdClass} text-ink-3 text-[0.6875rem]`}>{inst.lastPlayed}</td>
                        <td className={`${tdClass} text-right`} onClick={stop}>
                            <div className="inline-flex gap-1">
                                <Button
                                    color="success"
                                    size="sm"
                                    className="font-bold text-[0.6875rem]"
                                    startContent={<I.play size={10}/>}
                                >
                                    Play
                                </Button>
                                <Button
                                    isIconOnly
                                    variant="bordered"
                                    size="sm"
                                    aria-label="More"
                                    className="w-[26px] h-[26px] min-w-0"
                                >
                                    <I.more size={12}/>
                                </Button>
                            </div>
                        </td>
                    </tr>
                ))}
                </tbody>
            </table>
        </Card>
    );
}
