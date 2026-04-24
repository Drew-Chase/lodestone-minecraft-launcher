import {SelectSettingRow, SettingCard, ToggleRow} from "./primitives";

export default function GeneralPane() {
    return (
        <>
            <SettingCard>
                <div className="-m-1">
                    <SelectSettingRow
                        label="Default instance directory"
                        value="~/Lodestone/instances"
                    />
                    <SelectSettingRow label="Startup behavior" value="Show library"/>
                    <SelectSettingRow label="On game launch" value="Minimize launcher"/>
                    <SelectSettingRow label="After game exits" value="Restore launcher" last/>
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
                        on
                    />
                    <ToggleRow
                        label="Beta channel"
                        desc="Get new features early (may be unstable)"
                    />
                    <ToggleRow
                        label="Auto-update game versions"
                        desc="Pull new Minecraft releases automatically"
                        on
                        last
                    />
                </div>
            </SettingCard>

            <SettingCard
                title="Concurrent downloads"
                desc="How many files to fetch at once per instance."
            >
                <div className="flex items-center gap-3.5">
                    {/* Custom slider — absolute-positioned fill and handle on top of a
                        rounded track. The value is display-only for now. */}
                    <div className="relative flex-1 h-1.5 bg-line rounded-[3px]">
                        <div
                            className="absolute left-0 w-[55%] h-full rounded-[3px]"
                            style={{
                                background: "var(--mc-green)",
                                boxShadow: "0 0 10px var(--mc-green-glow)",
                            }}
                        />
                        <div
                            className="absolute w-4 h-4 rounded-full bg-white -top-[5px]"
                            style={{
                                left: "55%",
                                transform: "translateX(-50%)",
                                boxShadow:
                                    "0 2px 6px rgba(0,0,0,0.5), 0 0 0 3px rgba(34,255,132,0.25)",
                            }}
                        />
                    </div>
                    <div className="font-mono text-[0.8125rem] font-bold min-w-[28px] text-right">
                        11
                    </div>
                </div>
            </SettingCard>
        </>
    );
}
