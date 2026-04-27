import {Tab, Tabs} from "@heroui/react";
import {I} from "../shell/icons";
import {usePersistedState} from "../../hooks/usePersistedState";
import InstanceGrid from "./InstanceGrid";
import InstanceCompact from "./InstanceCompact";
import InstanceTable from "./InstanceTable";
import type {Instance, ViewMode} from "./instances";

const filterTabs = ["Recent", "Pinned", "Modded", "Vanilla"] as const;
type FilterTab = (typeof filterTabs)[number];

type Props = {
    instances: Instance[];
    onDeleteRequest: (inst: Instance) => void;
};

// "Your Instances" section — section header, filter tabs, view-mode toggle,
// and the currently-selected view (grid/compact/table). Both selections persist
// across launches via localStorage.
export default function InstanceList({instances, onDeleteRequest}: Props)
{
    const [viewMode, setViewMode] = usePersistedState<ViewMode>(
        "library.viewMode",
        "grid",
    );
    const [activeTab, setActiveTab] = usePersistedState<FilterTab>(
        "library.filterTab",
        "Recent",
    );

    return (
        <>
            {/* Section header */}
            <div className="flex items-center mb-3.5 gap-3">
                <div className="text-base font-bold tracking-tight">Your Instances</div>
                <div className="flex-1"/>

                {/* Filter tabs */}
                <Tabs
                    aria-label="Instance filter"
                    selectedKey={activeTab}
                    onSelectionChange={(key) => setActiveTab(key as FilterTab)}
                    variant="solid"
                    color="success"
                    size="sm"
                    classNames={{
                        tabList:
                            "bg-[rgba(255,255,255,0.04)] border border-line p-0.5 rounded-md gap-0.5",
                        cursor:
                            "bg-mc-green shadow-[0_0_0_1px_rgba(34,255,132,0.6),0_8px_20px_-8px_rgba(34,255,132,0.35)]",
                        tab: "h-7 px-3",
                        tabContent:
                            "text-xs font-medium text-ink-2 group-data-[selected=true]:text-bg-0"
                    }}
                >
                    {filterTabs.map((t) => (
                        <Tab key={t} title={t}/>
                    ))}
                </Tabs>

                {/* View-mode toggle */}
                <Tabs
                    aria-label="View mode"
                    selectedKey={viewMode}
                    onSelectionChange={(key) => setViewMode(key as ViewMode)}
                    variant="light"
                    size="md"
                    classNames={{
                        tabList:
                            "bg-[rgba(255,255,255,0.04)] border border-line p-1 rounded-md gap-0",
                        cursor:
                            "!bg-[rgba(34,255,132,0.18)] !shadow-[inset_0_0_0_1px_rgba(34,255,132,0.4)] !rounded-md",
                        tab: "h-8 px-3 data-[hover=true]:opacity-100",
                        tabContent:
                            "text-tiny font-semibold text-ink-2 group-data-[selected=true]:text-mc-green"
                    }}
                >
                    <Tab
                        key="grid"
                        title={
                            <span className="flex items-center gap-2">
                <I.grid size={18}/>Grid
              </span>
                        }
                    />
                    <Tab
                        key="compact"
                        title={
                            <span className="flex items-center gap-2">
                <I.list size={18}/>Compact
              </span>
                        }
                    />
                    <Tab
                        key="table"
                        title={
                            <span className="flex items-center gap-2">
                <I.table size={18}/>Table
              </span>
                        }
                    />
                </Tabs>
            </div>

            {/* Active view */}
            {viewMode === "grid" && <InstanceGrid list={instances} onDeleteRequest={onDeleteRequest}/>}
            {viewMode === "compact" && <InstanceCompact list={instances} onDeleteRequest={onDeleteRequest}/>}
            {viewMode === "table" && <InstanceTable list={instances} onDeleteRequest={onDeleteRequest}/>}
        </>
    );
}
