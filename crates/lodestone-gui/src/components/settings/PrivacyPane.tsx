import {Button} from "@heroui/react";
import {I} from "../shell/icons";
import {SettingCard, ToggleRow} from "./primitives";
import {useSettings} from "../../context/SettingsContext";

export default function PrivacyPane() {
    const {settings, update} = useSettings();

    return (
        <>
            <SettingCard
                title="Telemetry"
                desc="Help improve Lodestone by sending anonymous usage data."
            >
                <div className="-m-1">
                    <ToggleRow
                        label="Send crash reports"
                        desc="Stack traces with no personal data attached"
                        on={settings.crashReports}
                        onChange={(v) => update("crashReports", v)}
                    />
                    <ToggleRow
                        label="Anonymous usage stats"
                        desc="Feature usage counters only"
                        on={settings.usageStats}
                        onChange={(v) => update("usageStats", v)}
                    />
                    <ToggleRow
                        label="Performance diagnostics"
                        desc="FPS + launch times, per instance"
                        on={settings.performanceDiagnostics}
                        onChange={(v) => update("performanceDiagnostics", v)}
                        last
                    />
                </div>
            </SettingCard>

            <SettingCard
                title="Mod data permissions"
                desc="Mods can request access to your system."
            >
                <div className="-m-1">
                    <ToggleRow
                        label="Allow filesystem access"
                        desc="For mods with map import or screenshots"
                        on={settings.filesystemAccess}
                        onChange={(v) => update("filesystemAccess", v)}
                    />
                    <ToggleRow
                        label="Allow network requests"
                        desc="Online skins, online leaderboards"
                        on={settings.networkAccess}
                        onChange={(v) => update("networkAccess", v)}
                    />
                    <ToggleRow
                        label="Allow hardware access"
                        desc="Input devices, controllers, VR headsets"
                        on={settings.hardwareAccess}
                        onChange={(v) => update("hardwareAccess", v)}
                        last
                    />
                </div>
            </SettingCard>

            <SettingCard
                title="Data"
                desc="Export or clear everything stored locally."
            >
                <div className="flex gap-2.5 flex-wrap">
                    <Button
                        variant="bordered"
                        size="sm"
                        startContent={<I.download size={13}/>}
                    >
                        Export my data
                    </Button>
                    <Button
                        variant="bordered"
                        size="sm"
                        startContent={<I.refresh size={13}/>}
                    >
                        Clear cache · 1.2 GB
                    </Button>
                    <Button
                        variant="bordered"
                        size="sm"
                        className="text-[#ff6b6b] border-[rgba(255,107,107,0.3)]"
                        startContent={<I.x size={13}/>}
                    >
                        Delete all local data
                    </Button>
                </div>
            </SettingCard>
        </>
    );
}
