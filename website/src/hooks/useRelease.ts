import {createContext, useContext, useEffect, useState} from "react";

export interface ReleaseAsset {
    name: string;
    download_url: string;
    size: number;
    content_type: string;
    platform: string | null;
}

export interface PlatformDownload {
    platform: string;
    label: string;
    assets: ReleaseAsset[];
}

export interface Release {
    version: string;
    tag: string;
    name: string;
    changelog: string;
    prerelease: boolean;
    published_at: string | null;
    html_url: string;
    assets: ReleaseAsset[];
}

export interface LatestRelease {
    release: Release;
    downloads: PlatformDownload[];
}

export interface ReleaseData {
    loading: boolean;
    available: boolean;
    latest: LatestRelease | null;
    version: string;
    tag: string;
    publishedAt: string | null;
    /** Total install size of the primary asset for the detected OS, in bytes */
    installSize: number | null;
    /** Direct download URL for the detected OS */
    downloadUrl: string | null;
    /** Platform downloads grouped by OS */
    windows: PlatformDownload | null;
    macos: PlatformDownload | null;
    linux: PlatformDownload | null;
}

function detectOS(): "windows" | "macos" | "linux" {
    const ua = navigator.userAgent;
    if (ua.includes("Mac")) return "macos";
    if (ua.includes("Linux")) return "linux";
    return "windows";
}

function getOSLabel(os: "windows" | "macos" | "linux"): string {
    switch (os) {
        case "macos": return "macOS";
        case "linux": return "Linux";
        default: return "Windows";
    }
}

function pickPrimaryAsset(platform: PlatformDownload | null): ReleaseAsset | null {
    if (!platform) return null;
    const assets = platform.assets;
    // Prefer installer over portable
    const preferred = assets.find(a => {
        const n = a.name.toLowerCase();
        return n.endsWith("-setup.exe") || n.endsWith(".msi") || n.endsWith(".dmg") || n.endsWith(".appimage") || n.endsWith(".deb");
    });
    return preferred ?? assets[0] ?? null;
}

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(0)} MB`;
}

export function useRelease(): ReleaseData {
    const [loading, setLoading] = useState(true);
    const [latest, setLatest] = useState<LatestRelease | null>(null);

    useEffect(() => {
        fetch("/api/releases/latest")
            .then(res => {
                if (!res.ok) return null;
                return res.json();
            })
            .then((data: LatestRelease | null) => {
                setLatest(data);
            })
            .catch(() => {
                setLatest(null);
            })
            .finally(() => {
                setLoading(false);
            });
    }, []);

    const available = latest !== null;
    const os = detectOS();

    const windows = latest?.downloads.find(d => d.platform === "windows") ?? null;
    const macos = latest?.downloads.find(d => d.platform === "macos") ?? null;
    const linux = latest?.downloads.find(d => d.platform === "linux") ?? null;

    const currentPlatform = os === "windows" ? windows : os === "macos" ? macos : linux;
    const primary = pickPrimaryAsset(currentPlatform);

    return {
        loading,
        available,
        latest,
        version: latest?.release.version ?? "",
        tag: latest?.release.tag ?? "",
        publishedAt: latest?.release.published_at ?? null,
        installSize: primary?.size ?? null,
        downloadUrl: primary?.download_url ?? null,
        windows,
        macos,
        linux,
    };
}

export const ReleaseContext = createContext<ReleaseData>({
    loading: true,
    available: false,
    latest: null,
    version: "",
    tag: "",
    publishedAt: null,
    installSize: null,
    downloadUrl: null,
    windows: null,
    macos: null,
    linux: null,
});

export function useReleaseContext(): ReleaseData {
    return useContext(ReleaseContext);
}

export {detectOS, getOSLabel, pickPrimaryAsset, formatBytes};
