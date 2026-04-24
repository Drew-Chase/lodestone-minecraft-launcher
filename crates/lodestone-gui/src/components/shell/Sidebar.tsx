import {useLocation, useNavigate} from "react-router-dom";
import {I} from "./icons";

type NavId = "home" | "discover" | "worlds" | "servers" | "friends";

const items: {id: NavId; path: string; icon: keyof typeof I; label: string}[] = [
    {id: "home", path: "/", icon: "home", label: "Library"},
    {id: "discover", path: "/discover", icon: "compass", label: "Discover"},
    {id: "worlds", path: "/worlds", icon: "globe", label: "Worlds"},
    {id: "servers", path: "/servers", icon: "server", label: "Servers"},
    {id: "friends", path: "/friends", icon: "users", label: "Friends"},
];

// Determine active rail item from current pathname.
function activeIdFor(pathname: string): NavId | "downloads" | "settings" | null {
    if (pathname === "/") return "home";
    if (pathname.startsWith("/discover")) return "discover";
    if (pathname.startsWith("/worlds")) return "worlds";
    if (pathname.startsWith("/servers")) return "servers";
    if (pathname.startsWith("/friends")) return "friends";
    if (pathname.startsWith("/downloads")) return "downloads";
    if (pathname.startsWith("/settings")) return "settings";
    return null;
}

export default function Sidebar() {
    const location = useLocation();
    const navigate = useNavigate();
    const active = activeIdFor(location.pathname);

    return (
        <div
            className="flex-shrink-0 flex flex-col items-center relative border-r border-line bg-bg-1"
            style={{width: 68, padding: "18px 0", gap: 6}}
        >
            {/* Logo */}
            <div style={{marginBottom: 14, position: "relative"}}>
                <div
                    style={{
                        position: "absolute",
                        inset: -6,
                        borderRadius: 14,
                        background: "radial-gradient(circle, rgba(34,255,132,0.45) 0%, transparent 70%)",
                        filter: "blur(8px)",
                        pointerEvents: "none",
                    }}
                />
                <img
                    src="/lodestone-logo.svg"
                    alt="Lodestone"
                    width={40}
                    height={40}
                    style={{
                        position: "relative",
                        display: "block",
                        filter: "drop-shadow(0 0 6px rgba(34,255,132,0.5))",
                    }}
                />
            </div>
            <div className="div-line" style={{width: 32, marginBottom: 4}}/>

            {items.map((it) => {
                const IconC = I[it.icon];
                const isActive = active === it.id;
                return (
                    <div
                        key={it.id}
                        className={`nav-item${isActive ? " active" : ""}`}
                        onClick={() => navigate(it.path)}
                        title={it.label}
                    >
                        <IconC size={20}/>
                    </div>
                );
            })}

            <div style={{flex: 1}}/>

            <div
                className={`nav-item${active === "downloads" ? " active" : ""}`}
                onClick={() => navigate("/downloads")}
                title="Downloads"
            >
                <I.download size={20}/>
            </div>
            <div
                className={`nav-item${active === "settings" ? " active" : ""}`}
                onClick={() => navigate("/settings")}
                title="Settings"
            >
                <I.settings size={20}/>
            </div>

            {/* User avatar */}
            <div
                style={{
                    width: 40,
                    height: 40,
                    borderRadius: 10,
                    marginTop: 8,
                    background: "linear-gradient(135deg, #22ff84 0%, #0a5c33 100%)",
                    border: "2px solid rgba(34,255,132,0.6)",
                    boxShadow: "0 0 16px var(--mc-green-glow)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    fontSize: 14,
                    fontWeight: 800,
                    color: "#062814",
                    cursor: "pointer",
                    position: "relative",
                }}
            >
                EN
                <div
                    style={{
                        position: "absolute",
                        bottom: -1,
                        right: -1,
                        width: 10,
                        height: 10,
                        borderRadius: "50%",
                        background: "var(--mc-green)",
                        border: "2px solid var(--bg-1)",
                    }}
                />
            </div>
        </div>
    );
}
