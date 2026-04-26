import {useEffect, useState} from "react";
import {Navigate, useNavigate, useParams} from "react-router-dom";
import {Tab, Tabs} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import InstanceHero from "../components/instance/InstanceHero";
import OverviewTab from "../components/instance/OverviewTab";
import ModsTab from "../components/instance/ModsTab";
import ScreenshotsTab from "../components/instance/ScreenshotsTab";
import LogsTab from "../components/instance/LogsTab";
import WorldsTab from "../components/instance/WorldsTab";
import SettingsTab from "../components/instance/SettingsTab";
import {configToInstance, toSlug, type Instance} from "../components/library/instances";
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

interface InstanceConfig {
    id: number;
    name: string;
    minecraft_version: string;
    loader: string;
    loader_version: string | null;
    java_version: string | null;
    created_at: string;
    last_played: string | null;
    instance_path: string;
}

export default function InstanceDetail() {
    const {slug} = useParams<{slug: string}>();
    const navigate = useNavigate();
    const [tab, setTab] = usePersistedState<DetailTab>(
        `instanceDetail.tab.${slug ?? ""}`,
        "overview",
    );

    const [instance, setInstance] = useState<Instance | undefined>(undefined);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        if (!slug) {
            setLoading(false);
            return;
        }
        invoke<InstanceConfig[]>("list_instances")
            .then((configs) => {
                const all = configs.map(configToInstance);
                const found = all.find((i) => toSlug(i.name) === slug);
                setInstance(found);
            })
            .catch(() => setInstance(undefined))
            .finally(() => setLoading(false));
    }, [slug]);

    if (loading) {
        return (
            <div className="flex items-center justify-center flex-1 bg-bg-0">
                <div className="w-6 h-6 border-2 border-mc-green border-t-transparent rounded-full animate-spin"/>
            </div>
        );
    }

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
