import {createContext, useContext, useEffect, useState, useCallback, useRef, type ReactNode} from "react";
import {invoke} from "@tauri-apps/api/core";
import type {Settings} from "../types/settings";

const DEFAULT_SETTINGS: Settings = {
    instanceDir: "",
    startupBehavior: "Show library",
    onGameLaunch: "Minimize launcher",
    afterGameExits: "Restore launcher",
    autoUpdate: true,
    betaChannel: false,
    autoUpdateGames: true,
    concurrentDownloads: 11,
    maxMemoryMb: 8192,
    defaultJavaPath: "",
    jvmArguments: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200\n-XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch",
    accentColor: "#22ff84",
    theme: "void",
    animations: true,
    particles: true,
    glass: true,
    aurora: true,
    reduceMotion: false,
    font: "Inter",
    maxConcurrentDownloads: 8,
    modSourcePriority: "Modrinth → CurseForge",
    assetCdn: "Auto (closest)",
    connectionTimeout: 30,
    useSystemProxy: true,
    customProxyUrl: "",
    crashReports: true,
    usageStats: true,
    performanceDiagnostics: false,
    filesystemAccess: true,
    networkAccess: true,
    hardwareAccess: false,
};

interface SettingsContextValue {
    settings: Settings;
    loading: boolean;
    systemRamMb: number;
    dirty: boolean;
    update: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
    save: () => Promise<void>;
    reset: () => Promise<void>;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

export function useSettings(): SettingsContextValue {
    const ctx = useContext(SettingsContext);
    if (!ctx) throw new Error("useSettings must be used within <SettingsProvider>");
    return ctx;
}

export function SettingsProvider({children}: { children: ReactNode }) {
    const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
    const [loading, setLoading] = useState(true);
    const [systemRamMb, setSystemRamMb] = useState(16384);
    const [dirty, setDirty] = useState(false);
    const savedRef = useRef<Settings>(DEFAULT_SETTINGS);

    useEffect(() => {
        Promise.all([
            invoke<Settings>("get_settings"),
            invoke<number>("get_system_ram"),
        ])
            .then(([s, ram]) => {
                setSettings(s);
                savedRef.current = s;
                setSystemRamMb(ram);
            })
            .catch((e) => console.warn("settings load failed:", e))
            .finally(() => setLoading(false));
    }, []);

    const update = useCallback(<K extends keyof Settings>(key: K, value: Settings[K]) => {
        setSettings((prev) => {
            const next = {...prev, [key]: value};
            return next;
        });
        setDirty(true);
    }, []);

    const save = useCallback(async () => {
        try {
            await invoke("save_settings", {settings});
            savedRef.current = settings;
            setDirty(false);
        } catch (e) {
            console.error("settings save failed:", e);
        }
    }, [settings]);

    const reset = useCallback(async () => {
        try {
            const s = await invoke<Settings>("reset_settings");
            setSettings(s);
            savedRef.current = s;
            setDirty(false);
        } catch (e) {
            console.error("settings reset failed:", e);
        }
    }, []);

    return (
        <SettingsContext.Provider
            value={{settings, loading, systemRamMb, dirty, update, save, reset}}
        >
            {children}
        </SettingsContext.Provider>
    );
}
