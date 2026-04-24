import {Navigate, useNavigate, useParams} from "react-router-dom";
import {Tab, Tabs} from "@heroui/react";
import InstanceHero from "../components/instance/InstanceHero";
import OverviewTab from "../components/instance/OverviewTab";
import ModsTab from "../components/instance/ModsTab";
import ScreenshotsTab from "../components/instance/ScreenshotsTab";
import LogsTab from "../components/instance/LogsTab";
import WorldsTab from "../components/instance/WorldsTab";
import SettingsTab from "../components/instance/SettingsTab";
import {findInstanceBySlug} from "../components/library/instances";
import {usePersistedState} from "../hooks/usePersistedState";

type DetailTab = "overview" | "mods" | "worlds" | "screenshots" | "logs" | "settings";

const tabKeys: DetailTab[] = [
    "overview",
    "mods",
    "worlds",
    "screenshots",
    "logs",
    "settings",
];

// Instance detail page at /library/:slug. Hero banner at top with the instance's
// Scene + title + actions, underlined tabs below, and the active tab's content
// in the scroll area. Redirects to /library if the slug doesn't resolve to a
// known instance (e.g. typo in the URL).
//
// The currently-selected tab persists per-instance via usePersistedState keyed
// on the slug — switching between instances restores each one's last tab.
// Tabs that want a sticky toolbar (Mods, Worlds) take over the vertical space
// with their own flex layout; the content container is just `flex-1 min-h-0`
// with no overflow, so each tab controls scrolling internally.
export default function InstanceDetail() {
    const {slug} = useParams<{slug: string}>();
    const navigate = useNavigate();
    const [tab, setTab] = usePersistedState<DetailTab>(
        `instanceDetail.tab.${slug ?? ""}`,
        "overview",
    );

    const instance = slug ? findInstanceBySlug(slug) : undefined;
    if (!instance) {
        return <Navigate to="/library" replace/>;
    }

    return (
        <div className="flex flex-col flex-1 min-w-0 min-h-0 bg-bg-0">
            <InstanceHero instance={instance} onBack={() => navigate("/library")}/>

            {/* Tabs row — pt-[52px] clears the avatar that protrudes from the hero. */}
            <div className="pl-7 pr-7 pt-[52px] border-b border-line">
                <Tabs
                    aria-label="Instance sections"
                    selectedKey={tab}
                    onSelectionChange={(key) => setTab(key as DetailTab)}
                    variant="underlined"
                    color="success"
                    classNames={{
                        tabList: "gap-1 p-0 w-full relative rounded-none border-b-0",
                        cursor: "bg-mc-green",
                        tab: "h-10 px-4",
                        tabContent:
                            "text-[0.8125rem] font-medium capitalize text-ink-2 group-data-[selected=true]:text-mc-green",
                    }}
                >
                    {tabKeys.map((t) => (
                        <Tab key={t} title={t}/>
                    ))}
                </Tabs>
            </div>

            {/* Tab body — each tab component owns its own scroll so Mods / Worlds can
                pin their toolbar while the list scrolls. */}
            <div className="flex-1 min-h-0 flex flex-col">
                {tab === "overview" && <OverviewTab instance={instance}/>}
                {tab === "mods" && <ModsTab/>}
                {tab === "worlds" && <WorldsTab/>}
                {tab === "screenshots" && <ScreenshotsTab/>}
                {tab === "logs" && <LogsTab/>}
                {tab === "settings" && <SettingsTab/>}
            </div>
        </div>
    );
}
