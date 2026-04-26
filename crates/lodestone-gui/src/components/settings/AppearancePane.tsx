import {Card} from "@heroui/react";
import {I} from "../shell/icons";
import {cardSurfaceStyle} from "../surfaces";
import {SettingCard, ToggleRow} from "./primitives";
import {useSettings} from "../../context/SettingsContext";

const accents = ["#22ff84", "#9747ff", "#ffb545", "#47d9ff", "#ff5ec8", "#ff6b3d"];

type Theme = {id: string; name: string; bg: string};

const themes: Theme[] = [
    {id: "void", name: "Void", bg: "#08090a"},
    {id: "slate", name: "Slate", bg: "#14181f"},
    {id: "obsidian", name: "Obsidian", bg: "#1a0f1e"},
];

type Font = {name: string; desc: string};

const fonts: Font[] = [
    {name: "Inter", desc: "Geometric sans · default"},
    {name: "Space Grotesk", desc: "Rounded geometric · playful"},
];

export default function AppearancePane() {
    const {settings, update} = useSettings();

    return (
        <>
            <SettingCard
                title="Accent color"
                desc="Primary highlight used across the interface."
            >
                <div className="flex gap-2.5">
                    {accents.map((c) => {
                        const selected = settings.accentColor === c;
                        return (
                            <div
                                key={c}
                                className="w-11 h-11 rounded-xl cursor-pointer relative"
                                onClick={() => update("accentColor", c)}
                                style={{
                                    background: c,
                                    boxShadow: selected
                                        ? `0 0 0 2px var(--bg-0), 0 0 0 4px ${c}, 0 0 20px ${c}80`
                                        : "none",
                                }}
                            >
                                {selected && (
                                    <I.check
                                        size={16}
                                        className="absolute inset-0 m-auto text-black"
                                    />
                                )}
                            </div>
                        );
                    })}
                    <div className="w-11 h-11 rounded-xl cursor-pointer border-2 border-dashed border-line-strong flex items-center justify-center text-ink-3">
                        <I.plus size={16}/>
                    </div>
                </div>
            </SettingCard>

            <SettingCard title="Theme" desc="Global color scheme and surface treatment.">
                <div className="grid grid-cols-3 gap-2.5">
                    {themes.map((t) => {
                        const active = settings.theme === t.id;
                        return (
                            <div
                                key={t.id}
                                onClick={() => update("theme", t.id)}
                                className="h-[110px] rounded-xl cursor-pointer relative overflow-hidden border"
                                style={{
                                    background: t.bg,
                                    borderColor: active ? "var(--mc-green)" : "var(--line)",
                                    borderWidth: active ? 2 : 1,
                                    boxShadow: active ? "0 0 0 4px rgba(34,255,132,0.12)" : "none",
                                }}
                            >
                                {/* Mock header bar */}
                                <div
                                    className="absolute top-2.5 left-2.5 right-2.5 h-2 rounded-[3px]"
                                    style={{background: "rgba(255,255,255,0.08)"}}
                                />
                                {/* Mock accent tile */}
                                <div
                                    className="absolute w-[30px] h-[30px] rounded-md"
                                    style={{top: 24, left: 10, background: settings.accentColor}}
                                />
                                {/* Mock content panel */}
                                <div
                                    className="absolute h-[30px] rounded-md"
                                    style={{
                                        top: 24,
                                        left: 46,
                                        right: 10,
                                        background: "rgba(255,255,255,0.06)",
                                    }}
                                />
                                <div className="absolute bottom-2.5 left-2.5 text-[0.6875rem] font-bold">
                                    {t.name}
                                </div>
                            </div>
                        );
                    })}
                </div>
            </SettingCard>

            <Card
                className="p-1 mb-4 border border-line"
                style={cardSurfaceStyle}
            >
                <ToggleRow
                    label="Interface animations"
                    desc="Card hovers, page transitions, micro-interactions"
                    on={settings.animations}
                    onChange={(v) => update("animations", v)}
                />
                <ToggleRow
                    label="Ambient particles"
                    desc="Floating dust on hero banners"
                    on={settings.particles}
                    onChange={(v) => update("particles", v)}
                />
                <ToggleRow
                    label="Glass effect"
                    desc="Frosted blur on panels and popovers"
                    on={settings.glass}
                    onChange={(v) => update("glass", v)}
                />
                <ToggleRow
                    label="Aurora backgrounds"
                    desc="Soft color glows on idle screens"
                    on={settings.aurora}
                    onChange={(v) => update("aurora", v)}
                />
                <ToggleRow
                    label="Reduce motion"
                    desc="Respect system prefers-reduced-motion"
                    on={settings.reduceMotion}
                    onChange={(v) => update("reduceMotion", v)}
                    last
                />
            </Card>

            <SettingCard title="Typography" desc="Interface font.">
                <div className="grid grid-cols-2 gap-2.5">
                    {fonts.map((f) => {
                        const active = settings.font === f.name;
                        return (
                            <div
                                key={f.name}
                                onClick={() => update("font", f.name)}
                                className="p-3.5 rounded-[10px] cursor-pointer border"
                                style={{
                                    borderColor: active ? "var(--mc-green)" : "var(--line)",
                                    background: active ? "rgba(34,255,132,0.05)" : "transparent",
                                }}
                            >
                                <div
                                    className="text-2xl font-extrabold -tracking-[0.5px] mb-0.5"
                                    style={{fontFamily: f.name === "Inter" ? "Inter" : "'Space Grotesk'"}}
                                >
                                    Aa Bb Cc
                                </div>
                                <div className="text-xs font-semibold">{f.name}</div>
                                <div className="text-[0.6875rem] text-ink-3">{f.desc}</div>
                            </div>
                        );
                    })}
                </div>
            </SettingCard>
        </>
    );
}
