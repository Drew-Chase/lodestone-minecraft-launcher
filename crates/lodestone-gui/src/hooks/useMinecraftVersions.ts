import {useEffect, useState} from "react";
import {invoke} from "@tauri-apps/api/core";

export interface McVersion {
    id: string;
    version_type: string; // "Release" | "Snapshot" | "Old Beta" | "Old Alpha"
}

let cached: McVersion[] | null = null;

export function useMinecraftVersions() {
    const [versions, setVersions] = useState<McVersion[]>(cached ?? []);
    const [loading, setLoading] = useState(cached === null);

    useEffect(() => {
        if (cached) return;
        let cancelled = false;

        invoke<McVersion[]>("get_minecraft_versions")
            .then((v) => {
                cached = v;
                if (!cancelled) {
                    setVersions(v);
                    setLoading(false);
                }
            })
            .catch(() => {
                if (!cancelled) setLoading(false);
            });

        return () => { cancelled = true; };
    }, []);

    return {versions, loading};
}
