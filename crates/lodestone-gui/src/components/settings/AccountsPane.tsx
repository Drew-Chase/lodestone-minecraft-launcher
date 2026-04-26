import {Button} from "@heroui/react";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {SettingCard} from "./primitives";
import {useAuth} from "../../context/AuthContext";

function accountColor(mode: string): string {
    switch (mode) {
        case "microsoft": return "#22ff84";
        case "offline": return "#9747ff";
        case "demo": return "#ff9747";
        default: return "#22ff84";
    }
}

function accountTypeLabel(mode: string): string {
    switch (mode) {
        case "microsoft": return "Microsoft";
        case "offline": return "Offline";
        case "demo": return "Demo";
        default: return mode;
    }
}

function accountSubtext(mode: string): string {
    switch (mode) {
        case "microsoft": return "Microsoft account";
        case "offline": return "Offline profile";
        case "demo": return "Demo profile";
        default: return mode;
    }
}

export default function AccountsPane() {
    const {session} = useAuth();

    return (
        <>
            <SettingCard
                title="Signed-in accounts"
                desc="Switch between profiles with one click."
            >
                <div className="flex flex-col gap-2">
                    {session && (
                        <div
                            className="flex items-center gap-3.5 px-3.5 py-3 rounded-[10px] border"
                            style={{
                                borderColor: "rgba(34,255,132,0.4)",
                                background: "rgba(34,255,132,0.05)",
                            }}
                        >
                            <div
                                className="w-10 h-10 rounded-lg flex items-center justify-center font-extrabold text-sm"
                                style={{
                                    background: `linear-gradient(135deg, ${accountColor(session.mode)}, #0a5c33)`,
                                    color: "#062814",
                                }}
                            >
                                {session.username[0]}
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="text-[0.8125rem] font-bold flex items-center gap-2">
                                    {session.username}
                                    <Chip variant="green" className="text-[0.5625rem]">
                                        PRIMARY
                                    </Chip>
                                </div>
                                <div className="text-[0.6875rem] text-ink-3">
                                    {accountSubtext(session.mode)} · {accountTypeLabel(session.mode)}
                                </div>
                            </div>
                            <Button
                                isIconOnly
                                variant="bordered"
                                size="sm"
                                aria-label="More"
                                className="w-8 h-8 min-w-0"
                            >
                                <I.more size={14}/>
                            </Button>
                        </div>
                    )}
                    {!session && (
                        <div className="text-[0.8125rem] text-ink-3 py-4 text-center">
                            No account signed in.
                        </div>
                    )}
                    <Button
                        variant="bordered"
                        size="md"
                        className="mt-1.5 justify-center"
                        startContent={<I.plus size={13}/>}
                    >
                        Add Microsoft account
                    </Button>
                </div>
            </SettingCard>

            {/* The 3D voxel SkinBrowser from the design is out of scope for this pass
                (it's a large standalone component with pose animations). Placeholder
                card keeps the layout honest until we implement it. */}
            <SettingCard
                title="Skin browser"
                desc="Preview your skin in 3D and apply from saved looks."
            >
                <div className="flex flex-col items-center justify-center gap-2 py-10 text-ink-3">
                    <I.user size={24}/>
                    <div className="text-[0.8125rem] font-semibold text-ink-2">
                        3D skin browser coming soon
                    </div>
                    <div className="text-[0.6875rem]">
                        Drag-rotate character, pose animations, and saved skin library.
                    </div>
                </div>
            </SettingCard>
        </>
    );
}
