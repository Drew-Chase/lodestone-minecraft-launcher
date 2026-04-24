import {useState} from "react";
import {Button, Input, Tab, Tabs} from "@heroui/react";
import TitleBar from "../components/shell/TitleBar";
import {I} from "../components/shell/icons";
import WorldGrid from "../components/worlds/WorldGrid";
import WorldTable from "../components/worlds/WorldTable";
import {worlds, type World, type WorldViewMode} from "../components/worlds/worldsData";

type FilterKey = "all" | "pinned" | "survival" | "creative" | "hardcore";

const filterDefs: {id: FilterKey; label: string; match: (w: World) => boolean}[] = [
    {id: "all", label: "All worlds", match: () => true},
    {id: "pinned", label: "Pinned", match: (w) => !!w.pinned},
    {id: "survival", label: "Survival", match: (w) => w.gamemode === "Survival"},
    {id: "creative", label: "Creative", match: (w) => w.gamemode === "Creative"},
    {id: "hardcore", label: "Hardcore", match: (w) => w.gamemode === "Hardcore"},
];

export default function Worlds() {
    const [filter, setFilter] = useState<FilterKey>("all");
    const [mode, setMode] = useState<WorldViewMode>("grid");

    const activeDef = filterDefs.find((f) => f.id === filter) ?? filterDefs[0];
    const visible = worlds.filter(activeDef.match);

    return (
        <div className="flex flex-col flex-1 min-w-0 min-h-0 bg-bg-0">
            <TitleBar
                title="Worlds"
                subtitle={`${worlds.length} worlds across 6 instances · 3.4 GB total`}
            >
                <Input
                    placeholder="Search worlds by name or seed…"
                    size="sm"
                    classNames={{
                        base: "w-[320px]",
                        inputWrapper: "bg-[rgba(255,255,255,0.04)] border border-line",
                    }}
                    startContent={<I.search size={14}/>}
                />
                <Button
                    variant="bordered"
                    size="sm"
                    startContent={<I.upload size={14}/>}
                >
                    Import world
                </Button>
                <Button
                    color="success"
                    size="sm"
                    className="font-bold"
                    startContent={<I.plus size={14}/>}
                >
                    New world
                </Button>
            </TitleBar>

            <div className="flex-1 overflow-y-auto px-7 py-6">
                {/* Filter row — hand-rolled to match the design's neutral (non-green)
                    active state (bg-3 + line-strong border). */}
                <div className="flex items-center gap-2.5 mb-5">
                    <div className="flex gap-1">
                        {filterDefs.map((f) => {
                            const active = filter === f.id;
                            const count = worlds.filter(f.match).length;
                            return (
                                <button
                                    key={f.id}
                                    type="button"
                                    onClick={() => setFilter(f.id)}
                                    className={[
                                        "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold cursor-pointer border transition-colors",
                                        active
                                            ? "bg-bg-3 border-line-strong text-ink-0"
                                            : "bg-transparent border-transparent text-ink-2 hover:bg-[rgba(255,255,255,0.04)]",
                                    ].join(" ")}
                                >
                                    {f.label}
                                    <span className="text-[0.625rem] text-ink-3 font-mono">
                    {count}
                  </span>
                                </button>
                            );
                        })}
                    </div>

                    <div className="flex-1"/>

                    <div className="text-[0.6875rem] text-ink-3 font-mono tracking-[0.5px]">
                        LAST PLAYED ↓
                    </div>

                    {/* Grid/Table toggle — same HeroUI Tabs pattern as the Library page
                        so the segmented control looks consistent between screens. */}
                    <Tabs
                        aria-label="World view mode"
                        selectedKey={mode}
                        onSelectionChange={(k) => setMode(k as WorldViewMode)}
                        variant="light"
                        size="sm"
                        classNames={{
                            tabList:
                                "bg-[rgba(255,255,255,0.04)] border border-line p-[3px] rounded-[10px] gap-0",
                            cursor:
                                "!bg-[rgba(34,255,132,0.18)] !shadow-[inset_0_0_0_1px_rgba(34,255,132,0.4)] !rounded-lg",
                            tab: "h-8 px-2.5 data-[hover=true]:opacity-100",
                            tabContent:
                                "text-ink-2 group-data-[selected=true]:text-mc-green",
                        }}
                    >
                        <Tab key="grid" title={<I.grid size={13}/>}/>
                        <Tab key="table" title={<I.table size={13}/>}/>
                    </Tabs>
                </div>

                {mode === "grid" ? (
                    <WorldGrid list={visible}/>
                ) : (
                    <WorldTable list={visible}/>
                )}
            </div>
        </div>
    );
}
