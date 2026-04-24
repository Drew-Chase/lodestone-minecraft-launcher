import {useState} from "react";
import {Button} from "@heroui/react";
import TitleBar from "../components/shell/TitleBar";
import {I} from "../components/shell/icons";
import GeneralPane from "../components/settings/GeneralPane";
import AccountsPane from "../components/settings/AccountsPane";
import JavaPane from "../components/settings/JavaPane";
import AppearancePane from "../components/settings/AppearancePane";
import NetworkPane from "../components/settings/NetworkPane";
import PrivacyPane from "../components/settings/PrivacyPane";

type Section = "general" | "account" | "java" | "appearance" | "network" | "privacy";

type SectionDef = {
    id: Section;
    icon: keyof typeof I;
    label: string;
    title: string;
    subtitle: string;
};

const sections: SectionDef[] = [
    {id: "general", icon: "settings", label: "General", title: "General", subtitle: "Startup, defaults and downloads"},
    {id: "account", icon: "user", label: "Accounts", title: "Accounts", subtitle: "Microsoft and offline profiles"},
    {id: "java", icon: "cpu", label: "Java & Memory", title: "Java & Memory", subtitle: "Runtimes, heap allocation, JVM args"},
    {id: "appearance", icon: "image", label: "Appearance", title: "Appearance", subtitle: "Make Lodestone yours"},
    {id: "network", icon: "globe", label: "Network", title: "Network", subtitle: "Downloads, proxies, concurrency"},
    {id: "privacy", icon: "lock", label: "Privacy", title: "Privacy", subtitle: "Telemetry and data controls"},
];

export default function Settings() {
    const [section, setSection] = useState<Section>("general");
    const activeDef = sections.find((s) => s.id === section) ?? sections[0];

    return (
        <div className="flex flex-1 min-w-0 bg-bg-0">
            {/* Left sub-nav — 220px rail with icon + label items. Active gets a green
                tint background and a 2px green left rail. */}
            <div className="w-[220px] border-r border-line px-3.5 py-6 flex-shrink-0">
                <div className="text-[0.6875rem] text-ink-3 font-mono tracking-[0.1em] mb-2.5 pl-2.5">
                    SETTINGS
                </div>
                {sections.map((s) => {
                    const IconC = I[s.icon];
                    const active = section === s.id;
                    return (
                        <div
                            key={s.id}
                            onClick={() => setSection(s.id)}
                            className="flex items-center gap-2.5 px-3 py-2.5 rounded-lg cursor-pointer text-[0.8125rem] mb-0.5 transition-colors"
                            style={{
                                background: active ? "rgba(34,255,132,0.08)" : "transparent",
                                color: active ? "var(--mc-green)" : "var(--ink-2)",
                                borderLeft: active
                                    ? "2px solid var(--mc-green)"
                                    : "2px solid transparent",
                            }}
                        >
                            <IconC size={15}/> {s.label}
                        </div>
                    );
                })}
            </div>

            <div className="flex-1 flex flex-col min-w-0">
                <TitleBar title={activeDef.title} subtitle={activeDef.subtitle}>
                    <Button
                        variant="bordered"
                        size="sm"
                        startContent={<I.refresh size={13}/>}
                    >
                        Reset
                    </Button>
                    <Button color="success" size="sm" className="font-bold">
                        Save changes
                    </Button>
                </TitleBar>

                <div className="flex-1 overflow-y-auto px-8 pt-7 pb-10">
                    <div className="max-w-[820px]">
                        {section === "general" && <GeneralPane/>}
                        {section === "account" && <AccountsPane/>}
                        {section === "java" && <JavaPane/>}
                        {section === "appearance" && <AppearancePane/>}
                        {section === "network" && <NetworkPane/>}
                        {section === "privacy" && <PrivacyPane/>}
                    </div>
                </div>
            </div>
        </div>
    );
}
