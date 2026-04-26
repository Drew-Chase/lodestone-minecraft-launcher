import {useEffect, useRef, useState} from "react";
import {useNavigate} from "react-router-dom";
import {Button, Input} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {getVersion} from "@tauri-apps/api/app";
import Scene from "../components/shell/Scene";
import Particles from "../components/shell/Particles";
import WindowChrome from "../components/shell/WindowChrome";
import {I} from "../components/shell/icons";
import {useAuth} from "../context/AuthContext";
import type {McVersion} from "../hooks/useMinecraftVersions";

export default function Login() {
    const navigate = useNavigate();
    const {session, loginMicrosoft, loginOffline, loginDemo, error, clearError} = useAuth();
    const [hoverMs, setHoverMs] = useState(false);
    const [loading, setLoading] = useState<string | null>(null);
    const [showOfflineInput, setShowOfflineInput] = useState(false);
    const [offlineUsername, setOfflineUsername] = useState("");
    const [localError, setLocalError] = useState<string | null>(null);
    const offlineInputRef = useRef<HTMLInputElement>(null);

    // App version from tauri.conf.json.
    const [appVersion, setAppVersion] = useState("");
    useEffect(() => {
        getVersion().then(setAppVersion).catch(() => {});
    }, []);

    // Auth service status ping.
    const [authStatus, setAuthStatus] = useState<{ online: boolean; ping: number | null }>({online: false, ping: null});
    useEffect(() => {
        let cancelled = false;
        const check = async () => {
            try {
                const start = performance.now();
                await fetch("https://login.microsoftonline.com/consumers/v2.0/.well-known/openid-configuration", {method: "GET", mode: "no-cors"});
                const elapsed = Math.round(performance.now() - start);
                if (!cancelled) setAuthStatus({online: true, ping: elapsed});
            } catch {
                if (!cancelled) setAuthStatus({online: false, ping: null});
            }
        };
        check();
        return () => { cancelled = true; };
    }, []);

    // Latest 4 Minecraft releases.
    const [latestReleases, setLatestReleases] = useState<string[]>([]);
    useEffect(() => {
        let cancelled = false;
        invoke<McVersion[]>("get_minecraft_versions")
            .then((versions) => {
                if (cancelled) return;
                const releases = versions
                    .filter((v) => v.version_type === "Release")
                    .slice(0, 4)
                    .map((v) => v.id);
                setLatestReleases(releases);
            })
            .catch(() => {});
        return () => { cancelled = true; };
    }, []);

    // Redirect if already authenticated.
    useEffect(() => {
        if (session) navigate("/library", {replace: true});
    }, [session, navigate]);

    // Focus the offline input when it appears.
    useEffect(() => {
        if (showOfflineInput) offlineInputRef.current?.focus();
    }, [showOfflineInput]);

    const displayError = localError || error;

    const handleMicrosoft = async () => {
        setLoading("ms");
        setLocalError(null);
        try {
            await loginMicrosoft();
        } catch {
            // Error is set in context
        } finally {
            setLoading(null);
        }
    };

    const handleOfflineSubmit = async () => {
        const name = offlineUsername.trim();
        if (!name) {
            setLocalError("Please enter a username.");
            return;
        }
        if (name.length > 16) {
            setLocalError("Username must be 16 characters or less.");
            return;
        }
        if (!/^[a-zA-Z0-9_]+$/.test(name)) {
            setLocalError("Username may only contain letters, numbers, and underscores.");
            return;
        }
        setLoading("offline");
        setLocalError(null);
        try {
            await loginOffline(name);
        } catch {
            // Error is set in context
        } finally {
            setLoading(null);
        }
    };

    const handleDemo = async () => {
        setLoading("demo");
        setLocalError(null);
        try {
            await loginDemo();
        } catch {
            // Error is set in context
        } finally {
            setLoading(null);
        }
    };

    return (
        <div className="relative flex flex-col h-screen w-screen overflow-hidden bg-black">
            {/* Background layers */}
            <div
                className="absolute -inset-10 blur-[24px] scale-110"
                style={{filter: "blur(24px) saturate(1.2)"}}
            >
                <Scene biome="forest" seed={7}/>
            </div>
            <div
                className="absolute inset-0"
                style={{
                    background:
                        "linear-gradient(180deg, rgba(0,0,0,0.35) 0%, rgba(0,0,0,0.75) 100%)",
                }}
            />
            <div className="aurora opacity-70"/>
            <Particles count={24}/>

            {/* Foreground: chrome + page content */}
            <WindowChrome/>
            <div className="relative flex-1 min-h-0">
                {/* Language selector (top-left) */}
                <div className="absolute top-5 left-6 z-10">
                    <Button
                        variant="flat"
                        size="sm"
                        className="font-sans bg-[rgba(0,0,0,0.4)] text-ink-1 border border-line backdrop-blur-md"
                        startContent={<I.globe size={14}/>}
                        endContent={<I.chevDown size={12}/>}
                    >
                        <span className="text-xs font-medium">English · US</span>
                    </Button>
                </div>

                {/* Center glass card */}
                <div className="absolute inset-0 flex items-center justify-center z-[5]">
                    <div
                        className="w-[440px] px-9 pt-10 pb-8 rounded-[20px] border border-[rgba(255,255,255,0.09)] backdrop-blur-[30px] backdrop-saturate-150 relative overflow-hidden"
                        style={{
                            background:
                                "linear-gradient(180deg, rgba(21,24,28,0.85) 0%, rgba(14,16,18,0.92) 100%)",
                            boxShadow:
                                "0 30px 80px rgba(0,0,0,0.6), 0 0 0 1px rgba(34,255,132,0.1), inset 0 1px 0 rgba(255,255,255,0.08)",
                        }}
                    >
                        {/* Corner accent glow */}
                        <div
                            className="absolute -top-[60px] -right-[60px] w-[200px] h-[200px] pointer-events-none"
                            style={{
                                background:
                                    "radial-gradient(circle, var(--mc-green-glow) 0%, transparent 70%)",
                            }}
                        />

                        {/* Logo mark + wordmark */}
                        <div className="flex items-center gap-3 mb-7">
                            <div className="relative w-[52px] h-[52px]">
                                <div
                                    className="absolute -inset-2 rounded-2xl blur-[10px] pointer-events-none"
                                    style={{
                                        background:
                                            "radial-gradient(circle, rgba(34,255,132,0.55) 0%, transparent 70%)",
                                    }}
                                />
                                <img
                                    src="/lodestone-logo.svg"
                                    alt="Lodestone"
                                    width={52}
                                    height={52}
                                    className="relative block"
                                    style={{filter: "drop-shadow(0 0 8px rgba(34,255,132,0.6))"}}
                                />
                            </div>
                            <div>
                                <div
                                    className="text-[1.375rem] font-extrabold tracking-tight text-transparent bg-clip-text drop-shadow-[0_0_20px_rgba(34,255,132,0.25)]"
                                    style={{
                                        backgroundImage:
                                            "linear-gradient(180deg, #ffffff 0%, #c8fcdf 100%)",
                                    }}
                                >
                                    Lodestone
                                </div>
                                <div className="font-mono text-[0.6875rem] text-ink-3 tracking-[0.5px]">
                                    THIRD-PARTY LAUNCHER{appVersion ? ` · v${appVersion}` : ""}
                                </div>
                            </div>
                        </div>

                        <div
                            className="text-[1.625rem] font-extrabold tracking-tight mb-1.5 text-white drop-shadow-[0_0_24px_rgba(34,255,132,0.35)]">
                            Welcome back, <span className="text-mc-green">crafter</span>
                        </div>
                        <div className="text-[0.8125rem] text-ink-2 mb-7 leading-relaxed">
                            Sign in to sync your worlds, instances, and friends across devices.
                        </div>

                        {/* Error display */}
                        {displayError && (
                            <div
                                className="mb-4 px-3.5 py-2.5 rounded-lg border border-[rgba(255,107,107,0.3)] bg-[rgba(255,107,107,0.08)] text-[0.8125rem] text-[#ff8a8a] leading-relaxed flex items-start gap-2">
                                <I.shield size={16} className="flex-shrink-0 mt-0.5"/>
                                <div className="flex-1">{displayError}</div>
                                <button
                                    className="flex-shrink-0 text-ink-3 hover:text-ink-1 transition-colors bg-transparent border-none cursor-pointer p-0"
                                    onClick={() => {
                                        setLocalError(null);
                                        clearError();
                                    }}
                                >
                                    <I.close size={14}/>
                                </button>
                            </div>
                        )}

                        {/* Big Microsoft sign-in */}
                        <button
                            onMouseEnter={() => setHoverMs(true)}
                            onMouseLeave={() => setHoverMs(false)}
                            onClick={handleMicrosoft}
                            disabled={loading !== null}
                            className={[
                                "font-sans w-full px-[18px] py-3.5 rounded-md cursor-pointer font-bold text-sm flex items-center gap-3 justify-center transition-all mb-2.5 border",
                                loading !== null ? "opacity-60 cursor-not-allowed" : "",
                                hoverMs && loading === null
                                    ? "text-[#062814] border-[rgba(34,255,132,0.6)] shadow-mc-glow-strong"
                                    : "text-ink-0 border-[rgba(255,255,255,0.1)]",
                            ].join(" ")}
                            style={{
                                background: hoverMs && loading === null
                                    ? "linear-gradient(180deg, #2aff8c 0%, #16d96a 100%)"
                                    : "rgba(255,255,255,0.06)",
                            }}
                        >
                            <div className="grid grid-cols-2 gap-[2px] w-4 h-4">
                                <div className="bg-[#f25022]"/>
                                <div className="bg-[#7fba00]"/>
                                <div className="bg-[#00a4ef]"/>
                                <div className="bg-[#ffb900]"/>
                            </div>
                            {loading === "ms" ? "Waiting for Microsoft login…" : "Continue with Microsoft"}
                            {loading !== "ms" && <I.chevRight size={16} className="ml-auto"/>}
                            {loading === "ms" && (
                                <div
                                    className="ml-auto w-4 h-4 border-2 border-current border-t-transparent rounded-full animate-spin"/>
                            )}
                        </button>

                        <div className="flex items-center gap-3 my-[18px]">
                            <div className="flex-1 h-px bg-[rgba(255,255,255,0.08)]"/>
                            <div className="text-[0.625rem] text-ink-3 tracking-[1.5px] font-semibold">OR</div>
                            <div className="flex-1 h-px bg-[rgba(255,255,255,0.08)]"/>
                        </div>

                        {/* Offline mode: either show the button or the username input */}
                        {showOfflineInput ? (
                            <div className="flex gap-2 mb-2.5">
                                <Input
                                    ref={offlineInputRef}
                                    size="sm"
                                    variant="bordered"
                                    placeholder="Enter username"
                                    maxLength={16}
                                    value={offlineUsername}
                                    onValueChange={(v) => setOfflineUsername(v)}
                                    onKeyDown={(e) => {
                                        if (e.key === "Enter") handleOfflineSubmit();
                                        if (e.key === "Escape") {
                                            setShowOfflineInput(false);
                                            setOfflineUsername("");
                                            setLocalError(null);
                                        }
                                    }}
                                    isDisabled={loading !== null}
                                    classNames={{
                                        inputWrapper: "border-line bg-[rgba(255,255,255,0.04)]",
                                        input: "font-sans text-sm",
                                    }}
                                />
                                <Button
                                    variant="bordered"
                                    size="sm"
                                    className="font-sans px-4 h-10"
                                    isLoading={loading === "offline"}
                                    isDisabled={loading !== null}
                                    onPress={handleOfflineSubmit}
                                >
                                    Play
                                </Button>
                                <Button
                                    variant="bordered"
                                    size="sm"
                                    isIconOnly
                                    className="h-10 w-10 min-w-0"
                                    isDisabled={loading !== null}
                                    onPress={() => {
                                        setShowOfflineInput(false);
                                        setOfflineUsername("");
                                        setLocalError(null);
                                    }}
                                >
                                    <I.close size={14}/>
                                </Button>
                            </div>
                        ) : (
                            <div className="grid grid-cols-2 gap-2.5">
                                <Button
                                    variant="bordered"
                                    className="font-sans p-3 text-ink-1"
                                    startContent={<I.user size={15}/>}
                                    isDisabled={loading !== null}
                                    onPress={() => setShowOfflineInput(true)}
                                >
                                    Offline Mode
                                </Button>
                                <Button
                                    variant="bordered"
                                    className="font-sans p-3 text-ink-1"
                                    startContent={<I.shield size={15}/>}
                                    isLoading={loading === "demo"}
                                    isDisabled={loading !== null}
                                    onPress={handleDemo}
                                >
                                    Demo Mode
                                </Button>
                            </div>
                        )}

                        {/* Footer line */}
                        <div
                            className="mt-6 pt-[18px] border-t border-line text-[0.6875rem] text-ink-3 flex justify-between">
                            <span>Not affiliated with Mojang or Microsoft</span>
                            <span className="text-mc-green cursor-pointer">Learn more ↗</span>
                        </div>
                    </div>
                </div>

                {/* Bottom status bar */}
                <div
                    className="font-mono absolute bottom-[18px] left-6 right-6 flex items-center gap-3.5 z-[6] text-[0.6875rem] text-ink-3">
                    <span className="flex items-center gap-1.5">
                        <span
                            className="pulse-dot"
                            style={{
                                width: 6,
                                height: 6,
                                backgroundColor: authStatus.online ? undefined : "#ff6b6b",
                            }}
                        />
                        {authStatus.online ? "AUTH SERVICES ONLINE" : "AUTH SERVICES OFFLINE"}
                    </span>
                    {authStatus.ping !== null && (
                        <>
                            <span>·</span>
                            <span>{authStatus.ping}ms</span>
                        </>
                    )}
                    <div className="flex-1"/>
                    {latestReleases.length > 0 && (
                        <span>{latestReleases.join(" · ")} available</span>
                    )}
                </div>
            </div>
        </div>
    );
}
