import {Button} from "@heroui/react";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {SettingCard} from "./primitives";

type Account = {
    name: string;
    email: string;
    type: "Microsoft" | "Offline";
    primary?: boolean;
    color: string;
};

const accounts: Account[] = [
    {name: "Notch_37", email: "user@outlook.com", type: "Microsoft", primary: true, color: "#22ff84"},
    {name: "Steve", email: "Offline profile", type: "Offline", color: "#9747ff"},
    {name: "Alex_Dev", email: "dev@outlook.com", type: "Microsoft", color: "#47d9ff"},
];

export default function AccountsPane() {
    return (
        <>
            <SettingCard
                title="Signed-in accounts"
                desc="Switch between profiles with one click."
            >
                <div className="flex flex-col gap-2">
                    {accounts.map((a, i) => (
                        <div
                            key={i}
                            className="flex items-center gap-3.5 px-3.5 py-3 rounded-[10px] border"
                            style={{
                                borderColor: a.primary
                                    ? "rgba(34,255,132,0.4)"
                                    : "var(--line)",
                                background: a.primary
                                    ? "rgba(34,255,132,0.05)"
                                    : "rgba(255,255,255,0.02)",
                            }}
                        >
                            <div
                                className="w-10 h-10 rounded-lg flex items-center justify-center font-extrabold text-sm"
                                style={{
                                    background: `linear-gradient(135deg, ${a.color}, #0a5c33)`,
                                    color: "#062814",
                                }}
                            >
                                {a.name[0]}
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="text-[0.8125rem] font-bold flex items-center gap-2">
                                    {a.name}
                                    {a.primary && (
                                        <Chip variant="green" className="text-[0.5625rem]">
                                            PRIMARY
                                        </Chip>
                                    )}
                                </div>
                                <div className="text-[0.6875rem] text-ink-3">
                                    {a.email} · {a.type}
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
                    ))}
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
