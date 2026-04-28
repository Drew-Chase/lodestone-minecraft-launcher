import {createContext, useCallback, useContext, useEffect, useState, type ReactNode} from "react";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";

export interface InstallProgress {
    instanceId: number;
    instanceName: string;
    stage: string;
    stageLabel: string;
    progress: number;
    filesDone: number;
    filesTotal: number;
}

interface CompletedInstall {
    instanceId: number;
    instanceName: string;
    completedAt: number; // Date.now()
}

interface LaunchContextValue {
    runningInstances: Set<number>;
    installingInstances: Map<number, InstallProgress>;
    completedInstalls: CompletedInstall[];
    launchInstance: (id: number) => Promise<void>;
    stopInstance: (id: number) => Promise<void>;
    isRunning: (id: number) => boolean;
    isInstalling: (id: number) => boolean;
    clearCompleted: () => void;
}

const LaunchContext = createContext<LaunchContextValue | null>(null);

export function useLaunch(): LaunchContextValue {
    const ctx = useContext(LaunchContext);
    if (!ctx) throw new Error("useLaunch must be used within <LaunchProvider>");
    return ctx;
}

export function LaunchProvider({children}: { children: ReactNode }) {
    const [running, setRunning] = useState<Set<number>>(new Set());
    const [installing, setInstalling] = useState<Map<number, InstallProgress>>(new Map());
    const [completed, setCompleted] = useState<CompletedInstall[]>([]);

    // Sync running instances on mount
    useEffect(() => {
        invoke<number[]>("get_running_instances")
            .then((ids) => setRunning(new Set(ids)))
            .catch(() => {});
    }, []);

    // Listen for events
    useEffect(() => {
        const unlisteners: (() => void)[] = [];

        listen<InstallProgress>("install-progress", (event) => {
            setInstalling((prev) => {
                const next = new Map(prev);
                next.set(event.payload.instanceId, event.payload);
                return next;
            });
        }).then((u) => unlisteners.push(u));

        listen<number>("instance-started", (event) => {
            const id = event.payload;
            setRunning((prev) => new Set([...prev, id]));
            // Move from installing to completed
            setInstalling((prev) => {
                const progress = prev.get(id);
                if (progress) {
                    setCompleted((c) => {
                        if (c.some(x => x.instanceId === id)) return c;
                        return [{instanceId: id, instanceName: progress.instanceName, completedAt: Date.now()}, ...c].slice(0, 20);
                    });
                }
                const next = new Map(prev);
                next.delete(id);
                return next;
            });
        }).then((u) => unlisteners.push(u));

        listen<number>("instance-stopped", (event) => {
            setRunning((prev) => {
                const next = new Set(prev);
                next.delete(event.payload);
                return next;
            });
        }).then((u) => unlisteners.push(u));

        // Modpack install completed (not launched, just installed)
        listen<{instanceId: number; instanceName: string}>("install-completed", (event) => {
            const {instanceId, instanceName} = event.payload;
            setInstalling((prev) => {
                const next = new Map(prev);
                next.delete(instanceId);
                return next;
            });
            setCompleted((c) => {
                if (c.some(x => x.instanceId === instanceId)) return c;
                return [{instanceId, instanceName, completedAt: Date.now()}, ...c].slice(0, 20);
            });
        }).then((u) => unlisteners.push(u));

        return () => {
            for (const u of unlisteners) u();
        };
    }, []);

    const launchInstance = useCallback(async (id: number) => {
        try {
            await invoke("launch_instance", {instanceId: id});
        } catch (e) {
            console.error("Launch failed:", e);
            // Remove from installing on error
            setInstalling((prev) => {
                const next = new Map(prev);
                next.delete(id);
                return next;
            });
            throw e;
        }
    }, []);

    const stopInstance = useCallback(async (id: number) => {
        await invoke("stop_instance", {instanceId: id});
    }, []);

    const isRunning = useCallback((id: number) => running.has(id), [running]);
    const isInstalling = useCallback((id: number) => installing.has(id), [installing]);
    const clearCompleted = useCallback(() => setCompleted([]), []);

    return (
        <LaunchContext.Provider
            value={{
                runningInstances: running,
                installingInstances: installing,
                completedInstalls: completed,
                launchInstance,
                stopInstance,
                isRunning,
                isInstalling,
                clearCompleted,
            }}
        >
            {children}
        </LaunchContext.Provider>
    );
}
