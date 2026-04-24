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
        setTimeout(() => navigate("/library"), 250);
    };

    return (
        <div className="relative flex flex-col h-screen w-screen overflow-hidden bg-black">
            {/* Background layers — fill the entire window so the scene shows through the chrome too. */}
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
                            // Complex layered gradient + multi-stop box-shadow: keep inline.
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
                                    className="text-[22px] font-extrabold tracking-tight text-transparent bg-clip-text drop-shadow-[0_0_20px_rgba(34,255,132,0.25)]"
                                    style={{
                                        backgroundImage:
                                            "linear-gradient(180deg, #ffffff 0%, #c8fcdf 100%)",
                                    }}
                                >
                                    Lodestone
                                </div>
                                <div className="font-mono text-[11px] text-ink-3 tracking-[0.5px]">
                                    THIRD-PARTY LAUNCHER · v2.1.4
                                </div>
                            </div>
                        </div>

                        <div className="text-[26px] font-extrabold tracking-tight mb-1.5 text-white drop-shadow-[0_0_24px_rgba(34,255,132,0.35)]">
                            Welcome back, <span className="text-mc-green">crafter</span>
                        </div>
                        <div className="text-[13px] text-ink-2 mb-7 leading-relaxed">
                            Sign in to sync your worlds, instances, and friends across devices.
                        </div>

                        {/* Big Microsoft sign-in */}
                        <button
                            onMouseEnter={() => setHoverMs(true)}
                            onMouseLeave={() => setHoverMs(false)}
                            onClick={() => handleSignIn("ms")}
                            className={[
                                "font-sans w-full px-[18px] py-3.5 rounded-md cursor-pointer font-bold text-sm flex items-center gap-3 justify-center transition-all mb-2.5 border",
                                hoverMs
                                    ? "text-[#062814] border-[rgba(34,255,132,0.6)] shadow-mc-glow-strong"
                                    : "text-ink-0 border-[rgba(255,255,255,0.1)]",
                            ].join(" ")}
                            style={{
                                // Hover gradient swap: keep inline since Tailwind arbitrary gradients
                                // get unwieldy for the :hover transition.
                                background: hoverMs
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
                            {loading === "ms" ? "Opening Microsoft…" : "Continue with Microsoft"}
                            {loading !== "ms" && <I.chevRight size={16} className="ml-auto"/>}
                        </button>

                        <div className="flex items-center gap-3 my-[18px]">
                            <div className="flex-1 h-px bg-[rgba(255,255,255,0.08)]"/>
                            <div className="text-[10px] text-ink-3 tracking-[1.5px] font-semibold">OR</div>
                            <div className="flex-1 h-px bg-[rgba(255,255,255,0.08)]"/>
                        </div>

                        <div className="grid grid-cols-2 gap-2.5">
                            <Button
                                variant="bordered"
                                className="font-sans p-3 text-ink-1"
                                startContent={<I.user size={15}/>}
                                onPress={() => handleSignIn("offline")}
                            >
                                Offline Mode
                            </Button>
                            <Button
                                variant="bordered"
                                className="font-sans p-3 text-ink-1"
                                startContent={<I.shield size={15}/>}
                                onPress={() => handleSignIn("demo")}
                            >
                                Demo Mode
                            </Button>
                        </div>

                        {/* Footer line */}
                        <div className="mt-6 pt-[18px] border-t border-line text-[11px] text-ink-3 flex justify-between">
                            <span>Not affiliated with Mojang or Microsoft</span>
                            <span className="text-mc-green cursor-pointer">Learn more ↗</span>
                        </div>
                    </div>
                </div>

                {/* Bottom status bar */}
                <div className="font-mono absolute bottom-[18px] left-6 right-6 flex items-center gap-3.5 z-[6] text-[11px] text-ink-3">
          <span className="flex items-center gap-1.5">
            {/* pulse-dot base class hardcodes 8px; override via inline style for the status variant. */}
            <span className="pulse-dot" style={{width: 6, height: 6}}/>
            AUTH SERVICES ONLINE
          </span>
                    <span>·</span>
                    <span>124ms · NA-EAST</span>
                    <div className="flex-1"/>
                    <span>1.20.4 · 1.20.1 · 1.19.4 · 1.18.2 available</span>
                </div>
            </div>
        </div>
    );
}
