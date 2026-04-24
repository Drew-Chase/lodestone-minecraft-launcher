import {useState} from "react";
import {useNavigate} from "react-router-dom";
import {Button} from "@heroui/react";
import Scene from "../components/shell/Scene";
import Particles from "../components/shell/Particles";
import WindowChrome from "../components/shell/WindowChrome";
import {I} from "../components/shell/icons";

export default function Login() {
    const navigate = useNavigate();
    const [hoverMs, setHoverMs] = useState(false);
    const [loading, setLoading] = useState<string | null>(null);

    const handleSignIn = (method: string) => {
        setLoading(method);
        // No real auth yet — route to library after brief delay so the loading label is visible.
        setTimeout(() => navigate("/"), 250);
    };

    return (
        <div className="flex flex-col h-screen w-screen overflow-hidden" style={{background: "#000"}}>
            <WindowChrome/>
            <div className="relative flex-1 min-h-0" style={{background: "#000"}}>
                {/* Blurred forest scene banner */}
                <div
                    style={{
                        position: "absolute",
                        inset: "-40px",
                        filter: "blur(24px) saturate(1.2)",
                        transform: "scale(1.1)",
                    }}
                >
                    <Scene biome="forest" seed={7}/>
                </div>
                {/* Dim overlay */}
                <div
                    style={{
                        position: "absolute",
                        inset: 0,
                        background:
                            "linear-gradient(180deg, rgba(0,0,0,0.35) 0%, rgba(0,0,0,0.75) 100%)",
                    }}
                />
                {/* Aurora */}
                <div className="aurora" style={{opacity: 0.7}}/>
                <Particles count={24}/>

                {/* Language selector (top-left) */}
                <div style={{position: "absolute", top: 20, left: 24, zIndex: 10}}>
                    <Button
                        variant="flat"
                        size="sm"
                        className="font-sans"
                        style={{
                            background: "rgba(0,0,0,0.4)",
                            backdropFilter: "blur(12px)",
                            color: "var(--ink-1)",
                            border: "1px solid var(--line)",
                        }}
                        startContent={<I.globe size={14}/>}
                        endContent={<I.chevDown size={12}/>}
                    >
                        <span style={{fontSize: 12, fontWeight: 500}}>English · US</span>
                    </Button>
                </div>

                {/* Center glass card */}
                <div className="absolute inset-0 flex items-center justify-center" style={{zIndex: 5}}>
                    <div
                        style={{
                            width: 440,
                            background:
                                "linear-gradient(180deg, rgba(21,24,28,0.85) 0%, rgba(14,16,18,0.92) 100%)",
                            border: "1px solid rgba(255,255,255,0.09)",
                            borderRadius: 20,
                            padding: "40px 36px 32px",
                            backdropFilter: "blur(30px) saturate(1.4)",
                            boxShadow:
                                "0 30px 80px rgba(0,0,0,0.6), 0 0 0 1px rgba(34,255,132,0.1), inset 0 1px 0 rgba(255,255,255,0.08)",
                            position: "relative",
                            overflow: "hidden",
                        }}
                    >
                        {/* Corner accent glow */}
                        <div
                            style={{
                                position: "absolute",
                                top: -60,
                                right: -60,
                                width: 200,
                                height: 200,
                                background: "radial-gradient(circle, var(--mc-green-glow) 0%, transparent 70%)",
                                pointerEvents: "none",
                            }}
                        />

                        {/* Logo mark + wordmark */}
                        <div style={{display: "flex", alignItems: "center", gap: 12, marginBottom: 28}}>
                            <div style={{position: "relative", width: 52, height: 52}}>
                                <div
                                    style={{
                                        position: "absolute",
                                        inset: -8,
                                        borderRadius: 16,
                                        background:
                                            "radial-gradient(circle, rgba(34,255,132,0.55) 0%, transparent 70%)",
                                        filter: "blur(10px)",
                                        pointerEvents: "none",
                                    }}
                                />
                                <img
                                    src="/lodestone-logo.svg"
                                    alt="Lodestone"
                                    width={52}
                                    height={52}
                                    style={{
                                        position: "relative",
                                        display: "block",
                                        filter: "drop-shadow(0 0 8px rgba(34,255,132,0.6))",
                                    }}
                                />
                            </div>
                            <div>
                                <div
                                    style={{
                                        fontSize: 22,
                                        fontWeight: 800,
                                        letterSpacing: -0.5,
                                        background: "linear-gradient(180deg, #ffffff 0%, #c8fcdf 100%)",
                                        WebkitBackgroundClip: "text",
                                        backgroundClip: "text",
                                        WebkitTextFillColor: "transparent",
                                        textShadow: "0 0 20px rgba(34,255,132,0.25)",
                                    }}
                                >
                                    Lodestone
                                </div>
                                <div
                                    className="font-mono"
                                    style={{fontSize: 11, color: "var(--ink-3)", letterSpacing: 0.5}}
                                >
                                    THIRD-PARTY LAUNCHER · v2.1.4
                                </div>
                            </div>
                        </div>

                        <div
                            style={{
                                fontSize: 26,
                                fontWeight: 800,
                                letterSpacing: -0.6,
                                marginBottom: 6,
                                color: "#fff",
                                textShadow: "0 0 24px rgba(34,255,132,0.35)",
                            }}
                        >
                            Welcome back, <span style={{color: "var(--mc-green)"}}>crafter</span>
                        </div>
                        <div
                            style={{
                                fontSize: 13,
                                color: "var(--ink-2)",
                                marginBottom: 28,
                                lineHeight: 1.5,
                            }}
                        >
                            Sign in to sync your worlds, instances, and friends across devices.
                        </div>

                        {/* Big Microsoft sign-in */}
                        <button
                            onMouseEnter={() => setHoverMs(true)}
                            onMouseLeave={() => setHoverMs(false)}
                            onClick={() => handleSignIn("ms")}
                            className="font-sans"
                            style={{
                                width: "100%",
                                padding: "14px 18px",
                                borderRadius: 12,
                                cursor: "pointer",
                                background: hoverMs
                                    ? "linear-gradient(180deg, #2aff8c 0%, #16d96a 100%)"
                                    : "rgba(255,255,255,0.06)",
                                color: hoverMs ? "#062814" : "var(--ink-0)",
                                fontWeight: 700,
                                fontSize: 14,
                                display: "flex",
                                alignItems: "center",
                                gap: 12,
                                justifyContent: "center",
                                transition: "all 0.15s ease",
                                border:
                                    "1px solid " +
                                    (hoverMs ? "rgba(34,255,132,0.6)" : "rgba(255,255,255,0.1)"),
                                boxShadow: hoverMs
                                    ? "0 12px 30px -8px var(--mc-green-glow)"
                                    : "none",
                                marginBottom: 10,
                            }}
                        >
                            <div
                                style={{
                                    display: "grid",
                                    gridTemplateColumns: "1fr 1fr",
                                    gap: 2,
                                    width: 16,
                                    height: 16,
                                }}
                            >
                                <div style={{background: "#f25022"}}/>
                                <div style={{background: "#7fba00"}}/>
                                <div style={{background: "#00a4ef"}}/>
                                <div style={{background: "#ffb900"}}/>
                            </div>
                            {loading === "ms" ? "Opening Microsoft…" : "Continue with Microsoft"}
                            {loading !== "ms" && (
                                <I.chevRight size={16} style={{marginLeft: "auto"}}/>
                            )}
                        </button>

                        <div
                            style={{
                                display: "flex",
                                alignItems: "center",
                                gap: 12,
                                margin: "18px 0",
                            }}
                        >
                            <div style={{flex: 1, height: 1, background: "rgba(255,255,255,0.08)"}}/>
                            <div
                                style={{
                                    fontSize: 10,
                                    color: "var(--ink-3)",
                                    letterSpacing: 1.5,
                                    fontWeight: 600,
                                }}
                            >
                                OR
                            </div>
                            <div style={{flex: 1, height: 1, background: "rgba(255,255,255,0.08)"}}/>
                        </div>

                        <div style={{display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10}}>
                            <Button
                                variant="bordered"
                                className="font-sans"
                                style={{padding: "12px", color: "var(--ink-1)"}}
                                startContent={<I.user size={15}/>}
                                onPress={() => handleSignIn("offline")}
                            >
                                Offline Mode
                            </Button>
                            <Button
                                variant="bordered"
                                className="font-sans"
                                style={{padding: "12px", color: "var(--ink-1)"}}
                                startContent={<I.shield size={15}/>}
                                onPress={() => handleSignIn("demo")}
                            >
                                Demo Mode
                            </Button>
                        </div>

                        {/* Footer line */}
                        <div
                            style={{
                                marginTop: 24,
                                paddingTop: 18,
                                borderTop: "1px solid var(--line)",
                                fontSize: 11,
                                color: "var(--ink-3)",
                                display: "flex",
                                justifyContent: "space-between",
                            }}
                        >
                            <span>Not affiliated with Mojang or Microsoft</span>
                            <span style={{color: "var(--mc-green)", cursor: "pointer"}}>
                Learn more ↗
              </span>
                        </div>
                    </div>
                </div>

                {/* Bottom status bar */}
                <div
                    className="font-mono"
                    style={{
                        position: "absolute",
                        bottom: 18,
                        left: 24,
                        right: 24,
                        display: "flex",
                        alignItems: "center",
                        gap: 14,
                        zIndex: 6,
                        fontSize: 11,
                        color: "var(--ink-3)",
                    }}
                >
          <span style={{display: "flex", alignItems: "center", gap: 6}}>
            <span className="pulse-dot" style={{width: 6, height: 6}}/>
            AUTH SERVICES ONLINE
          </span>
                    <span>·</span>
                    <span>124ms · NA-EAST</span>
                    <div style={{flex: 1}}/>
                    <span>1.20.4 · 1.20.1 · 1.19.4 · 1.18.2 available</span>
                </div>
            </div>
        </div>
    );
}
