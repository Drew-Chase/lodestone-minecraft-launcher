import {Slider} from "@heroui/react";
import {SelectSettingRow, SettingCard, ToggleRow} from "./primitives";
import {useSettings} from "../../context/SettingsContext";

export default function GeneralPane() {
    const {settings, update} = useSettings();

    return (
        <>
            <SettingCard>
                <div className="-m-1">
                    <SelectSettingRow
                        label="Default instance directory"
                        value={settings.instanceDir || "Loading..."}
                    />
                    <SelectSettingRow
                        label="Startup behavior"
                        value={settings.startupBehavior}
                        options={["Show library", "Show last instance", "Show discover"]}
                        onChange={(v) => update("startupBehavior", v)}
                    />
                    <SelectSettingRow
                        label="On game launch"
                        value={settings.onGameLaunch}
                        options={["Minimize launcher", "Keep open", "Close launcher"]}
                        onChange={(v) => update("onGameLaunch", v)}
                    />
                    <SelectSettingRow
                        label="After game exits"
                        value={settings.afterGameExits}
                        options={["Restore launcher", "Keep minimized", "Close launcher"]}
                        onChange={(v) => update("afterGameExits", v)}
                        last
                    />
                </div>
            </SettingCard>

            <SettingCard
                title="Updates"
                desc="How Lodestone updates itself and Minecraft."
            >
                <div className="-m-1">
                    <ToggleRow
                        label="Auto-update Lodestone"
                        desc="Install stable releases in the background"
                        on={settings.autoUpdate}
                        onChange={(v) => update("autoUpdate", v)}
                    />
                    <ToggleRow
                        label="Beta channel"
                        desc="Get new features early (may be unstable)"
                        on={settings.betaChannel}
                        onChange={(v) => update("betaChannel", v)}
                    />
                    <ToggleRow
                        label="Auto-update game versions"
                        desc="Pull new Minecraft releases automatically"
                        on={settings.autoUpdateGames}
                        onChange={(v) => update("autoUpdateGames", v)}
                        last
                    />
                </div>
            </SettingCard>

            <SettingCard
                title="Concurrent downloads"
                desc="How many files to fetch at once per instance."
            >
                <Slider
                    minValue={1}
                    maxValue={20}
                    step={1}
                    value={settings.concurrentDownloads}
                    onChange={(v) => update("concurrentDownloads", v as number)}
                    aria-label="Concurrent downloads"
                    showTooltip
                    classNames={{
                        track: "bg-line h-1.5",
                        filler: "bg-mc-green",
                        thumb: "bg-white w-4 h-4 shadow-[0_2px_6px_rgba(0,0,0,0.5),0_0_0_3px_rgba(34,255,132,0.25)]",
                    }}
                    renderValue={({children}) => (
                        <div className="font-mono text-[0.8125rem] font-bold min-w-[28px] text-right">
                            {children}
                        </div>
                    )}
                />
            </SettingCard>
        </>
    );
}
