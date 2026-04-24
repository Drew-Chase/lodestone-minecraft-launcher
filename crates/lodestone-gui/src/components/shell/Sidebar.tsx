import {useState, type ReactNode} from "react";
import {useLocation, useNavigate} from "react-router-dom";
import {Popover, PopoverContent, PopoverTrigger, Tooltip} from "@heroui/react";
import {I} from "./icons";

type NavId = "home" | "discover" | "worlds" | "servers" | "friends";

const items: {id: NavId; path: string; icon: keyof typeof I; label: string}[] = [
    {id: "home", path: "/library", icon: "home", label: "Library"},
    {id: "discover", path: "/discover", icon: "compass", label: "Discover"},
    {id: "worlds", path: "/worlds", icon: "globe", label: "Worlds"},
    {id: "servers", path: "/servers", icon: "server", label: "Servers"},
    {id: "friends", path: "/friends", icon: "users", label: "Friends"},
];

// Determine active rail item from current pathname.
function activeIdFor(pathname: string): NavId | "downloads" | "settings" | null {
    if (pathname.startsWith("/library")) return "home";
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
    const [menuOpen, setMenuOpen] = useState(false);
    const active = activeIdFor(location.pathname);

    // Run an action then close the avatar popover.
    const runThenClose = (fn: () => void) => () => {
        setMenuOpen(false);
        fn();
    };

    return (
        <div className="flex-shrink-0 flex flex-col items-center relative border-r border-line bg-bg-1 w-[68px] py-[18px] gap-1.5">
            {/* Logo */}
            <div className="relative mb-3.5">
                <div
                    className="absolute -inset-1.5 rounded-[14px] blur-lg pointer-events-none"
                    style={{
                        background: "radial-gradient(circle, rgba(34,255,132,0.45) 0%, transparent 70%)",
                    }}
                />
                <img
                    src="/lodestone-logo.svg"
                    alt="Lodestone"
                    width={40}
                    height={40}
                    className="relative block"
                    style={{filter: "drop-shadow(0 0 6px rgba(34,255,132,0.5))"}}
                />
            </div>
            <div className="div-line w-8 mb-1"/>

            {items.map((it) => {
                const IconC = I[it.icon];
                const isActive = active === it.id;
                return (
                    <Tooltip key={it.id} content={it.label} placement="right" delay={250} offset={14}>
                        <div
                            className={`nav-item${isActive ? " active" : ""}`}
                            onClick={() => navigate(it.path)}
                        >
                            <IconC size={20}/>
                        </div>
                    </Tooltip>
                );
            })}

            <div className="flex-1"/>

            <Tooltip content="Downloads" placement="right" delay={250} offset={14}>
                <div
                    className={`nav-item${active === "downloads" ? " active" : ""}`}
                    onClick={() => navigate("/downloads")}
                >
                    <I.download size={20}/>
                </div>
            </Tooltip>
            <Tooltip content="Settings" placement="right" delay={250} offset={14}>
                <div
                    className={`nav-item${active === "settings" ? " active" : ""}`}
                    onClick={() => navigate("/settings")}
                >
                    <I.settings size={20}/>
                </div>
            </Tooltip>

            {/* User avatar with popover menu */}
            <Popover
                isOpen={menuOpen}
                onOpenChange={setMenuOpen}
                placement="right-end"
                offset={12}
                backdrop="transparent"
            >
                <PopoverTrigger>
                    <button
                        type="button"
                        aria-label="User menu"
                        className="relative mt-2 w-10 h-10 rounded-md flex items-center justify-center text-sm font-extrabold cursor-pointer p-0 border-2 shadow-mc-glow"
                        style={{
                            // Gradient + border color stay inline (design tokens + alpha combos
                            // not cleanly expressible as single Tailwind utilities).
                            background: "linear-gradient(135deg, #22ff84 0%, #0a5c33 100%)",
                            borderColor: "rgba(34,255,132,0.6)",
                            color: "#062814",
                        }}
                    >
                        EN
                        <div className="absolute -bottom-px -right-px w-2.5 h-2.5 rounded-full bg-mc-green border-2 border-bg-1"/>
                    </button>
                </PopoverTrigger>
                <PopoverContent
                    className="p-0 border border-line-strong min-w-[240px]"
                    style={{
                        background:
                            "linear-gradient(180deg, rgba(21,24,28,0.95) 0%, rgba(14,16,18,0.98) 100%)",
                        boxShadow: "0 20px 40px rgba(0,0,0,0.5), 0 0 0 1px rgba(34,255,132,0.08)",
                    }}
                >
                    <UserMenu
                        onProfile={runThenClose(() => {
                            /* TODO: profile page */
                        })}
                        onSettings={runThenClose(() => navigate("/settings"))}
                        onSignOut={runThenClose(() => navigate("/"))}
                    />
                </PopoverContent>
            </Popover>
        </div>
    );
}

function UserMenu({
                      onProfile,
                      onSettings,
                      onSignOut,
                  }: {
    onProfile: () => void;
    onSettings: () => void;
    onSignOut: () => void;
}) {
    return (
        <div className="p-2 min-w-[240px]">
            {/* Header */}
            <div className="flex items-center gap-2.5 px-2 py-2.5">
                <div
                    className="flex-shrink-0 w-9 h-9 rounded-md flex items-center justify-center text-xs font-extrabold border-2"
                    style={{
                        background: "linear-gradient(135deg, #22ff84 0%, #0a5c33 100%)",
                        borderColor: "rgba(34,255,132,0.6)",
                        color: "#062814",
                    }}
                >
                    EN
                </div>
                <div className="min-w-0 flex-1 text-left">
                    <div className="text-[0.8125rem] font-bold text-ink-1 tracking-tight">
                        Not signed in
                    </div>
                    <div className="font-mono text-[0.625rem] text-ink-3 mt-0.5">OFFLINE MODE</div>
                </div>
            </div>

            <div className="div-line my-1.5"/>

            <MenuItem icon={<I.user size={14}/>} label="Profile" onClick={onProfile}/>
            <MenuItem
                icon={<I.settings size={14}/>}
                label="Account Settings"
                onClick={onSettings}
            />

            <div className="div-line my-1.5"/>

            <MenuItem
                icon={<I.external size={14}/>}
                label="Sign out"
                onClick={onSignOut}
                danger
            />
        </div>
    );
}

function MenuItem({
                      icon,
                      label,
                      onClick,
                      danger = false,
                  }: {
    icon: ReactNode;
    label: string;
    onClick: () => void;
    danger?: boolean;
}) {
    return (
        <button
            type="button"
            onClick={onClick}
            className={[
                "flex items-center gap-2.5 w-full px-2.5 py-2 bg-transparent border-none rounded-lg text-[0.8125rem] font-medium cursor-pointer text-left transition-colors font-sans",
                danger
                    ? "text-[#ff6b6b] hover:bg-[rgba(255,107,107,0.1)]"
                    : "text-ink-1 hover:bg-[rgba(255,255,255,0.04)]",
            ].join(" ")}
        >
      <span
          className={`w-[18px] flex items-center justify-center ${danger ? "text-[#ff6b6b]" : "text-ink-3"}`}
      >
        {icon}
      </span>
            <span className="flex-1">{label}</span>
        </button>
    );
}
