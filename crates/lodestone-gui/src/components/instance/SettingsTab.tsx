import {useCallback, useEffect, useRef, useState, type ReactNode} from "react";
import {Button, Slider} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import Chip from "../Chip";
import {Switch} from "../Switch";
import {I} from "../shell/icons";
import {cardSurfaceStyle} from "../surfaces";
import {useSettings} from "../../context/SettingsContext";
import {useMinecraftVersions} from "../../hooks/useMinecraftVersions";
import {VersionDropdown} from "../modals/primitives";
import type {Instance} from "../library/instances";

type Props = {
    instance: Instance;
    onDeleteRequest?: () => void;
};

interface InstanceSettings {
    maxMemoryMb: number | null;
    javaPath: string | null;
    jvmArguments: string | null;
    windowMode: string | null;
    resolutionWidth: number | null;
    resolutionHeight: number | null;
    hideLauncher: boolean | null;
    quitAfterGame: boolean | null;
}

const emptySettings: InstanceSettings = {
    maxMemoryMb: null,
    javaPath: null,
    jvmArguments: null,
    windowMode: null,
    resolutionWidth: null,
    resolutionHeight: null,
    hideLauncher: null,
    quitAfterGame: null,
};

type NavKey = "java" | "window" | "version" | "advanced" | "danger";

type NavItem = {
    k: NavKey;
    label: string;
    icon: (p: { size?: number; className?: string }) => ReactNode;
    danger?: boolean;
};

const navItems: NavItem[] = [
    {k: "java", label: "Java & Memory", icon: I.cpu},
    {k: "window", label: "Window", icon: I.image},
    {k: "version", label: "Game Version", icon: I.tag},
    {k: "advanced", label: "Advanced", icon: I.settings},
    {k: "danger", label: "Danger Zone", icon: I.trash, danger: true},
];

interface LoaderVersionsResponse {
    versions: string[];
    recommended: string | null;
}

const loaderKeys = ["vanilla", "fabric", "forge", "neoforge", "quilt"] as const;
const loaderLabels: Record<string, string> = {
    vanilla: "Vanilla", fabric: "Fabric", forge: "Forge", neoforge: "NeoForge", quilt: "Quilt",
};

export default function SettingsTab({instance, onDeleteRequest}: Props) {
    const [active, setActive] = useState<NavKey>("java");
    const {settings: global, systemRamMb} = useSettings();
    const [local, setLocal] = useState<InstanceSettings>(emptySettings);
    const [dirty, setDirty] = useState(false);
    const scrollRef = useRef<HTMLDivElement>(null);
    const sectionRefs = useRef<Record<NavKey, HTMLDivElement | null>>({
        java: null, window: null, version: null, advanced: null, danger: null,
    });
    const isClickScrolling = useRef(false);

    // Version editing state
    const {versions: mcVersions, loading: mcLoading} = useMinecraftVersions();
    const [mcVersion, setMcVersion] = useState(instance.mc);
    const [loader, setLoader] = useState(instance.loader.toLowerCase());
    const [loaderVersion, setLoaderVersion] = useState(instance.loaderVersion ?? "");
    const [javaVersion, setJavaVersion] = useState(instance.javaVersion ?? "");
    const [loaderVersions, setLoaderVersions] = useState<string[]>([]);
    const [loaderRecommended, setLoaderRecommended] = useState<string | null>(null);
    const [loaderLoading, setLoaderLoading] = useState(false);
    const [versionDirty, setVersionDirty] = useState(false);

    // Fetch loader versions when loader or MC version changes
    useEffect(() => {
        if (loader === "vanilla") {
            setLoaderVersions([]);
            setLoaderRecommended(null);
            return;
        }
        setLoaderLoading(true);
        invoke<LoaderVersionsResponse>("get_loader_versions", {
            loader,
            minecraftVersion: mcVersion,
        })
            .then((r) => {
                setLoaderVersions(r.versions);
                setLoaderRecommended(r.recommended);
            })
            .catch(() => {
                setLoaderVersions([]);
                setLoaderRecommended(null);
            })
            .finally(() => setLoaderLoading(false));
    }, [loader, mcVersion]);

    // Fetch java version for selected MC version
    useEffect(() => {
        invoke<{majorVersion: number; label: string}>("get_java_for_version", {
            minecraftVersion: mcVersion,
        })
            .then((info) => setJavaVersion(String(info.majorVersion)))
            .catch(() => {});
    }, [mcVersion]);

    const saveVersionChanges = async () => {
        await invoke("update_instance", {
            request: {
                id: instance.id,
                minecraftVersion: mcVersion,
                loader,
                loaderVersion: loader === "vanilla" ? null : loaderVersion || null,
                javaVersion: javaVersion || null,
            },
        });
        setVersionDirty(false);
        window.location.reload();
    };

    const fetchSettings = useCallback(async () => {
        try {
            const s = await invoke<InstanceSettings>("get_instance_settings", {
                instancePath: instance.instancePath,
            });
            setLocal(s);
        } catch {
            setLocal(emptySettings);
        }
    }, [instance.instancePath]);

    useEffect(() => {
        fetchSettings();
    }, [fetchSettings]);

    const update = <K extends keyof InstanceSettings>(key: K, value: InstanceSettings[K]) => {
        setLocal((prev) => ({...prev, [key]: value}));
        setDirty(true);
    };

    const clearOverride = <K extends keyof InstanceSettings>(key: K) => {
        setLocal((prev) => ({...prev, [key]: null}));
        setDirty(true);
    };

    const save = async () => {
        await invoke("save_instance_settings", {
            instancePath: instance.instancePath,
            settings: local,
        });
        setDirty(false);
    };

    // Scroll-spy: observe which section is in view and update active sidebar
    useEffect(() => {
        const container = scrollRef.current;
        if (!container) return;

        const observer = new IntersectionObserver(
            (entries) => {
                if (isClickScrolling.current) return;
                for (const entry of entries) {
                    if (entry.isIntersecting) {
                        const key = entry.target.getAttribute("data-section") as NavKey | null;
                        if (key) setActive(key);
                    }
                }
            },
            {
                root: container,
                rootMargin: "-10% 0px -70% 0px",
                threshold: 0,
            },
        );

        for (const key of Object.keys(sectionRefs.current) as NavKey[]) {
            const el = sectionRefs.current[key];
            if (el) observer.observe(el);
        }

        return () => observer.disconnect();
    }, []);

    // Sidebar click: scroll to section
    const scrollToSection = (key: NavKey) => {
        setActive(key);
        const el = sectionRefs.current[key];
        if (!el || !scrollRef.current) return;
        isClickScrolling.current = true;
        el.scrollIntoView({behavior: "smooth", block: "start"});
        // Re-enable scroll-spy after animation
        setTimeout(() => {
            isClickScrolling.current = false;
        }, 600);
    };

    const overrideCount = Object.values(local).filter((v) => v !== null).length;

    return (
        <div className="flex-1 overflow-hidden flex">
            {/* Sidebar — fixed, does not scroll */}
            <div className="w-[200px] flex-shrink-0 px-4 pt-5 pb-10 self-start sticky top-0">
                {navItems.map((item) => {
                    const Icon = item.icon;
                    const isActive = active === item.k;
                    return (
                        <div
                            key={item.k}
                            onClick={() => scrollToSection(item.k)}
                            className="flex items-center gap-2.5 px-2.5 py-2 rounded-md text-xs mb-0.5 cursor-pointer transition-colors"
                            style={{
                                background: isActive ? "rgba(34,255,132,0.08)" : "transparent",
                                color: isActive
                                    ? "var(--mc-green)"
                                    : item.danger
                                        ? "#ff5a7a"
                                        : "var(--ink-2)",
                                borderLeft: isActive ? "2px solid var(--mc-green)" : "2px solid transparent",
                            }}
                        >
                            <Icon size={12}/>
                            <span>{item.label}</span>
                        </div>
                    );
                })}

                <div className="mt-[18px] p-3 bg-bg-1 border border-line rounded-lg">
                    <div className="font-mono text-[0.625rem] text-ink-3 tracking-[0.5px] mb-1.5">
                        OVERRIDES
                    </div>
                    <div className="text-[0.6875rem] text-ink-2 leading-snug">
                        {overrideCount === 0
                            ? "All settings inherited from global defaults."
                            : `${overrideCount} setting${overrideCount > 1 ? "s" : ""} overridden.`}
                    </div>
                </div>

                {(dirty || versionDirty) && (
                    <Button
                        color="success"
                        size="sm"
                        className="mt-3 w-full font-bold"
                        onPress={() => {
                            if (dirty) save();
                            if (versionDirty) saveVersionChanges();
                        }}
                    >
                        Save Changes
                    </Button>
                )}
            </div>

            {/* Scrollable content — all sections rendered */}
            <div ref={scrollRef} className="flex-1 overflow-y-auto px-4 pt-5 pb-10">
                {/* Java & Memory */}
                <div ref={(el) => { sectionRefs.current.java = el; }} data-section="java">
                    <Section icon={I.cpu} title="Java & Memory" desc="Runtime and heap allocation for this instance.">
                        <OverrideRow
                            label="Allocated memory"
                            hint={`Global default: ${formatMb(global.maxMemoryMb)}. System RAM: ${formatMb(systemRamMb)}.`}
                            overridden={local.maxMemoryMb !== null}
                            onClear={() => clearOverride("maxMemoryMb")}
                        >
                            <div className="w-[280px]">
                                <Slider
                                    size="sm"
                                    step={256}
                                    minValue={512}
                                    maxValue={systemRamMb}
                                    value={local.maxMemoryMb ?? global.maxMemoryMb}
                                    onChange={(v) => update("maxMemoryMb", v as number)}
                                    color="success"
                                    className="max-w-full"
                                    renderThumb={(props) => (
                                        <div {...props} className="group p-0.5 top-1/2 bg-mc-green rounded-full cursor-grab shadow-[0_0_8px_rgba(34,255,132,0.5)]">
                                            <span className="block w-2.5 h-2.5 rounded-full bg-mc-green"/>
                                        </div>
                                    )}
                                />
                                <div className="text-right font-mono text-xs text-mc-green font-semibold mt-1">
                                    {formatMb(local.maxMemoryMb ?? global.maxMemoryMb)}
                                </div>
                            </div>
                        </OverrideRow>
                        <OverrideRow
                            label="JVM arguments"
                            hint="Custom Java arguments for this instance."
                            overridden={local.jvmArguments !== null}
                            onClear={() => clearOverride("jvmArguments")}
                        >
                            <textarea
                                value={local.jvmArguments ?? global.jvmArguments}
                                onChange={(e) => update("jvmArguments", e.target.value)}
                                rows={3}
                                className="w-[300px] rounded-md border border-line p-2.5 font-mono text-[0.6875rem] leading-relaxed text-ink-1 bg-bg-0 outline-none resize-none"
                            />
                        </OverrideRow>
                    </Section>
                </div>

                {/* Window */}
                <div ref={(el) => { sectionRefs.current.window = el; }} data-section="window">
                    <Section icon={I.image} title="Window" desc="How Minecraft appears when this instance launches.">
                        <OverrideRow
                            label="Launch mode"
                            overridden={local.windowMode !== null}
                            onClear={() => clearOverride("windowMode")}
                        >
                            <div className="flex gap-0.5 p-0.5 bg-bg-1 border border-line rounded-md">
                                {["Windowed", "Borderless", "Fullscreen"].map((m) => (
                                    <div
                                        key={m}
                                        onClick={() => update("windowMode", m)}
                                        className={[
                                            "px-2.5 py-1 text-[0.6875rem] rounded-sm cursor-pointer",
                                            (local.windowMode ?? "Windowed") === m ? "bg-bg-2 text-ink-0" : "bg-transparent text-ink-3",
                                        ].join(" ")}
                                    >
                                        {m}
                                    </div>
                                ))}
                            </div>
                        </OverrideRow>
                        <OverrideRow
                            label="Resolution"
                            overridden={local.resolutionWidth !== null || local.resolutionHeight !== null}
                            onClear={() => {
                                clearOverride("resolutionWidth");
                                clearOverride("resolutionHeight");
                            }}
                        >
                            <div className="flex items-center gap-1.5">
                                <input
                                    value={local.resolutionWidth ?? 1920}
                                    onChange={(e) => update("resolutionWidth", parseInt(e.target.value) || 1920)}
                                    className="w-[72px] bg-bg-1 border border-line rounded-md px-2 py-1 text-[0.71875rem] text-ink-0 font-mono text-center outline-none"
                                />
                                <span className="text-ink-3 text-[0.6875rem]">×</span>
                                <input
                                    value={local.resolutionHeight ?? 1080}
                                    onChange={(e) => update("resolutionHeight", parseInt(e.target.value) || 1080)}
                                    className="w-[72px] bg-bg-1 border border-line rounded-md px-2 py-1 text-[0.71875rem] text-ink-0 font-mono text-center outline-none"
                                />
                            </div>
                        </OverrideRow>
                        <OverrideRow
                            label="Hide launcher while playing"
                            overridden={local.hideLauncher !== null}
                            onClear={() => clearOverride("hideLauncher")}
                        >
                            <Switch
                                isSelected={local.hideLauncher ?? true}
                                onValueChange={(v) => update("hideLauncher", v)}
                                size="sm"
                                color="success"
                            />
                        </OverrideRow>
                        <OverrideRow
                            label="Quit launcher after game exits"
                            overridden={local.quitAfterGame !== null}
                            onClear={() => clearOverride("quitAfterGame")}
                        >
                            <Switch
                                isSelected={local.quitAfterGame ?? false}
                                onValueChange={(v) => update("quitAfterGame", v)}
                                size="sm"
                                color="success"
                            />
                        </OverrideRow>
                    </Section>
                </div>

                {/* Game Version */}
                <div ref={(el) => { sectionRefs.current.version = el; }} data-section="version">
                    <Section icon={I.tag} title="Game Version" desc="Change Minecraft or mod-loader versions.">
                        <Row label="Minecraft version">
                            <div className="w-[220px]">
                                <VersionDropdown
                                    versions={mcVersions.filter((v) => v.version_type === "Release").map((v) => v.id)}
                                    selected={mcVersion}
                                    onChange={(v) => {
                                        setMcVersion(v);
                                        setVersionDirty(true);
                                    }}
                                    latestVersion={mcVersions.find((v) => v.version_type === "Release")?.id}
                                    loading={mcLoading}
                                />
                            </div>
                        </Row>
                        <Row label="Mod loader">
                            <div className="flex gap-1 p-0.5 bg-bg-1 border border-line rounded-md">
                                {loaderKeys.map((l) => (
                                    <div
                                        key={l}
                                        onClick={() => {
                                            setLoader(l);
                                            setLoaderVersion("");
                                            setVersionDirty(true);
                                        }}
                                        className={[
                                            "px-2.5 py-1 text-[0.6875rem] rounded-sm cursor-pointer",
                                            loader === l ? "bg-bg-2 text-ink-0" : "bg-transparent text-ink-3",
                                        ].join(" ")}
                                    >
                                        {loaderLabels[l]}
                                    </div>
                                ))}
                            </div>
                        </Row>
                        {loader !== "vanilla" && (
                            <Row label="Loader version">
                                <div className="w-[220px]">
                                    <VersionDropdown
                                        versions={loaderVersions}
                                        selected={loaderVersion}
                                        onChange={(v) => {
                                            setLoaderVersion(v);
                                            setVersionDirty(true);
                                        }}
                                        latestVersion={loaderRecommended ?? undefined}
                                        loading={loaderLoading}
                                        placeholder="Select loader version"
                                    />
                                </div>
                            </Row>
                        )}
                        <Row label="Java version" hint="Automatically detected from Minecraft version.">
                            <span className="font-mono text-sm text-ink-0">
                                {javaVersion ? `Java ${javaVersion}` : "Auto-detect"}
                            </span>
                        </Row>
                    </Section>
                </div>

                {/* Advanced */}
                <div ref={(el) => { sectionRefs.current.advanced = el; }} data-section="advanced">
                    <Section icon={I.settings} title="Advanced" desc="Power-user options. Change with care.">
                        <Row label="Instance path" hint="Location on disk for this instance.">
                            <div className="font-mono text-[0.6875rem] text-ink-2 max-w-[300px] truncate">
                                {instance.instancePath}
                            </div>
                        </Row>
                        <Row label="Instance ID">
                            <span className="font-mono text-sm text-ink-2">{instance.id}</span>
                        </Row>
                        <Row label="Created">
                            <span className="font-mono text-sm text-ink-2">
                                {instance.createdAt ? new Date(instance.createdAt).toLocaleDateString() : "—"}
                            </span>
                        </Row>
                    </Section>
                </div>

                {/* Danger Zone */}
                <div ref={(el) => { sectionRefs.current.danger = el; }} data-section="danger">
                    <DangerZone onDeleteRequest={onDeleteRequest}/>
                </div>
            </div>
        </div>
    );
}

function formatMb(mb: number): string {
    return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${mb} MB`;
}

// --- layout primitives ---

function Section({icon: Icon, title, desc, children}: {
    icon: (p: { size?: number; className?: string }) => ReactNode;
    title: string;
    desc: string;
    children: ReactNode;
}) {
    return (
        <div className="mb-7">
            <div className="flex items-start gap-3 mb-1">
                <div className="w-7 h-7 rounded-md bg-bg-1 border border-line flex items-center justify-center text-mc-green flex-shrink-0">
                    <Icon size={13}/>
                </div>
                <div className="flex-1">
                    <div className="text-[0.8125rem] font-semibold -tracking-[0.1px]">{title}</div>
                    <div className="text-[0.6875rem] text-ink-3 mt-0.5">{desc}</div>
                </div>
            </div>
            <div className="rounded-lg border border-line mt-3.5 px-[18px]" style={cardSurfaceStyle}>
                {children}
            </div>
        </div>
    );
}

function Row({label, hint, children}: {
    label: string;
    hint?: string;
    children: ReactNode;
}) {
    return (
        <div className="flex items-center gap-4 py-3.5 border-b border-line last:border-b-0">
            <div className="flex-1 min-w-0">
                <div className={`text-[0.78125rem] text-ink-0 ${hint ? "mb-[3px]" : ""}`}>{label}</div>
                {hint && <div className="text-[0.6875rem] text-ink-3 leading-snug">{hint}</div>}
            </div>
            <div className="flex-shrink-0 flex items-center gap-2">{children}</div>
        </div>
    );
}

function OverrideRow({label, hint, overridden, onClear, children}: {
    label: string;
    hint?: string;
    overridden: boolean;
    onClear: () => void;
    children: ReactNode;
}) {
    return (
        <div className="flex items-center gap-4 py-3.5 border-b border-line last:border-b-0">
            <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                    <span className={`text-[0.78125rem] ${overridden ? "text-ink-0" : "text-ink-2"}`}>{label}</span>
                    {overridden && (
                        <button
                            type="button"
                            onClick={onClear}
                            className="text-[0.5625rem] text-mc-green hover:underline cursor-pointer"
                        >
                            Reset to global
                        </button>
                    )}
                </div>
                {hint && <div className="text-[0.6875rem] text-ink-3 leading-snug">{hint}</div>}
            </div>
            <div className="flex-shrink-0 flex items-center gap-2">{children}</div>
        </div>
    );
}

function DangerZone({onDeleteRequest}: { onDeleteRequest?: () => void }) {
    const rows: { t: string; d: string; btn: string; danger?: boolean; action?: () => void }[] = [
        {
            t: "Reinstall game files",
            d: "Redownload vanilla Minecraft jars for this version. Mods and worlds are preserved.",
            btn: "Reinstall",
        },
        {
            t: "Reset instance settings",
            d: "Revert this instance back to global defaults.",
            btn: "Reset",
        },
        {
            t: "Delete instance",
            d: "Permanently remove this instance and all of its worlds, mods, and screenshots.",
            btn: "Delete",
            danger: true,
            action: onDeleteRequest,
        },
    ];

    return (
        <div>
            <div className="flex items-start gap-3 mb-1">
                <div
                    className="w-7 h-7 rounded-md bg-bg-1 border border-line flex items-center justify-center flex-shrink-0"
                    style={{color: "#ff5a7a"}}
                >
                    <I.trash size={13}/>
                </div>
                <div className="flex-1">
                    <div className="text-[0.8125rem] font-semibold -tracking-[0.1px]">Danger Zone</div>
                    <div className="text-[0.6875rem] text-ink-3 mt-0.5">
                        Destructive actions. These cannot be undone.
                    </div>
                </div>
            </div>
            <div
                className="rounded-lg p-[18px] mt-3.5 border"
                style={{...cardSurfaceStyle, borderColor: "rgba(255,90,122,0.2)"}}
            >
                {rows.map((r, i) => (
                    <div
                        key={r.t}
                        className={`flex items-center gap-4 py-3 ${i < rows.length - 1 ? "border-b border-line" : ""}`}
                    >
                        <div className="flex-1">
                            <div
                                className="text-[0.78125rem] mb-[3px]"
                                style={{color: r.danger ? "#ff5a7a" : "var(--ink-0)"}}
                            >
                                {r.t}
                            </div>
                            <div className="text-[0.6875rem] text-ink-3 leading-snug">{r.d}</div>
                        </div>
                        <button
                            type="button"
                            onClick={r.action}
                            className="px-3 py-1.5 text-[0.6875rem] rounded-md cursor-pointer border"
                            style={
                                r.danger
                                    ? {
                                        background: "rgba(255,90,122,0.1)",
                                        borderColor: "rgba(255,90,122,0.3)",
                                        color: "#ff5a7a",
                                    }
                                    : {
                                        background: "var(--bg-1)",
                                        borderColor: "var(--line)",
                                        color: "var(--ink-1)",
                                    }
                            }
                        >
                            {r.btn}
                        </button>
                    </div>
                ))}
            </div>
        </div>
    );
}
