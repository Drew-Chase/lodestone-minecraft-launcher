import Chip from "../Chip";
import {SelectSettingRow, SettingCard, ToggleRow} from "./primitives";
import {useSettings} from "../../context/SettingsContext";

export default function NetworkPane() {
    const {settings, update} = useSettings();

    return (
        <>
            <SettingCard
                title="Download speed"
                desc="Live, averaged over the last hour."
            >
                <div className="flex items-baseline gap-2.5 mb-3.5">
                    <div
                        className="text-[2.25rem] font-extrabold font-mono text-mc-green"
                        style={{textShadow: "0 0 18px var(--mc-green-glow)"}}
                    >
                        42.8
                    </div>
                    <div className="text-[0.8125rem] text-ink-3">MB/s</div>
                    <div className="flex-1"/>
                    <Chip variant="green">
                        <span className="pulse-dot" style={{width: 5, height: 5}}/> CONNECTED
                    </Chip>
                </div>
                <svg viewBox="0 0 300 50" className="w-full h-[50px]">
                    <defs>
                        <linearGradient id="net-spark" x1="0" x2="0" y1="0" y2="1">
                            <stop offset="0" stopColor="#22ff84" stopOpacity="0.4"/>
                            <stop offset="1" stopColor="#22ff84" stopOpacity="0"/>
                        </linearGradient>
                    </defs>
                    <path
                        d="M0 35 L15 28 L30 32 L45 22 L60 26 L75 16 L90 20 L105 14 L120 24 L135 18 L150 10 L165 22 L180 16 L195 28 L210 20 L225 12 L240 24 L255 18 L270 8 L285 16 L300 12 L300 50 L0 50 Z"
                        fill="url(#net-spark)"
                    />
                    <path
                        d="M0 35 L15 28 L30 32 L45 22 L60 26 L75 16 L90 20 L105 14 L120 24 L135 18 L150 10 L165 22 L180 16 L195 28 L210 20 L225 12 L240 24 L255 18 L270 8 L285 16 L300 12"
                        stroke="var(--mc-green)"
                        strokeWidth="1.5"
                        fill="none"
                    />
                </svg>
            </SettingCard>

            <SettingCard>
                <div className="-m-1">
                    <SelectSettingRow
                        label="Max concurrent downloads"
                        value={String(settings.maxConcurrentDownloads)}
                        options={["4", "8", "12", "16", "20"]}
                        onChange={(v) => update("maxConcurrentDownloads", parseInt(v, 10))}
                    />
                    <SelectSettingRow
                        label="Mod source priority"
                        value={settings.modSourcePriority}
                        options={["Modrinth → CurseForge", "CurseForge → Modrinth", "Modrinth only", "CurseForge only"]}
                        onChange={(v) => update("modSourcePriority", v)}
                    />
                    <SelectSettingRow
                        label="Asset CDN"
                        value={settings.assetCdn}
                        options={["Auto (closest)", "US East", "US West", "Europe", "Asia"]}
                        onChange={(v) => update("assetCdn", v)}
                    />
                    <SelectSettingRow
                        label="Connection timeout"
                        value={`${settings.connectionTimeout} s`}
                        options={["10 s", "15 s", "30 s", "60 s", "120 s"]}
                        onChange={(v) => update("connectionTimeout", parseInt(v, 10))}
                        last
                    />
                </div>
            </SettingCard>

            <SettingCard title="Proxy" desc="For restricted or corporate networks.">
                <div className="-m-1">
                    <ToggleRow
                        label="Use system proxy"
                        desc="Follow the OS network configuration"
                        on={settings.useSystemProxy}
                        onChange={(v) => update("useSystemProxy", v)}
                    />
                    <ToggleRow
                        label="Custom HTTP proxy"
                        desc="Override with proxy.yourorg.com:3128"
                        on={!!settings.customProxyUrl}
                        onChange={(v) => {
                            if (!v) update("customProxyUrl", "");
                        }}
                        last
                    />
                </div>
            </SettingCard>
        </>
    );
}
