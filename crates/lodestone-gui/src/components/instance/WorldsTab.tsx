import {useState} from "react";
import {Button, Card, Tab, Tabs} from "@heroui/react";
import {I} from "../shell/icons";
import {cardSurfaceStyle} from "../surfaces";
import WorldGrid from "../worlds/WorldGrid";
import WorldTable from "../worlds/WorldTable";
import {worlds, type WorldViewMode} from "../worlds/worldsData";
import type {Instance} from "../library/instances";

type Props = {
    instance: Instance;
};

// Per-instance worlds view. Filters the global worlds dataset by instance name.
// Reuses the same grid/table components the top-level /worlds page uses, just
// without the filter-tab row (already scoped) and with a tighter header.
export default function WorldsTab({instance}: Props) {
    const [mode, setMode] = useState<WorldViewMode>("grid");
    const visible = worlds.filter((w) => w.inst === instance.name);

    if (visible.length === 0) {
        // Empty state — matches the design's "No saved worlds yet" placeholder but
        // with a CTA to create one.
        return (
            <Card
                className="p-10 text-center border border-line"
                style={cardSurfaceStyle}
            >
                <div className="flex flex-col items-center gap-3 text-ink-3">
                    <I.globe size={28}/>
                    <div className="text-sm text-ink-2 font-semibold">No saved worlds yet</div>
                    <div className="text-xs">
                        Worlds you create in this instance will appear here.
                    </div>
                    <Button
                        color="success"
                        size="sm"
                        className="font-bold mt-2"
                        startContent={<I.plus size={13}/>}
                    >
                        New world
                    </Button>
                </div>
            </Card>
        );
    }

    return (
        <>
            <div className="flex items-center gap-2.5 mb-4">
                <div className="text-[0.6875rem] text-ink-3 font-mono tracking-[0.5px]">
                    {visible.length} SAVED WORLD{visible.length === 1 ? "" : "S"}
                </div>
                <div className="flex-1"/>
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
        </>
    );
}
