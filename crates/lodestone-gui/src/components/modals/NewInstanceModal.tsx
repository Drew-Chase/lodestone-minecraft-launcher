import ModalShell from "./ModalShell";
import {FooterBtn, InputRow, Label, SelectRow, VersionPicker} from "./primitives";
import {I} from "../shell/icons";

type Props = {isOpen: boolean; onClose: () => void};

const loaders = [
    {id: "vanilla", name: "Vanilla", color: "#9aa4ae", desc: "Unmodded Minecraft"},
    {id: "fabric", name: "Fabric", color: "#c9b88c", desc: "Lightweight, fast startup"},
    {id: "forge", name: "Forge", color: "#5a7fb3", desc: "Largest mod ecosystem"},
    {id: "neoforge", name: "NeoForge", color: "#e08548", desc: "Modern Forge fork"},
    {id: "quilt", name: "Quilt", color: "#c16fa3", desc: "Fabric-compatible"},
];

const templates = [
    {icon: I.sword, label: "Performance", mods: 24},
    {icon: I.pickaxe, label: "Tech & Auto", mods: 86},
    {icon: I.shield, label: "Adventure", mods: 45},
    {icon: I.image, label: "Shaders + RP", mods: 12},
];

// Fabric is pre-selected in the design — matches the instance the user is likely
// building on the hero card.
const SELECTED_LOADER = "fabric";

export default function NewInstanceModal({isOpen, onClose}: Props) {
    return (
        <ModalShell
            isOpen={isOpen}
            onClose={onClose}
            title="New Instance"
            subtitle="Spin up a fresh Minecraft profile. Pick version and loader, we'll do the rest."
            icon={I.plus}
            size="3xl"
            footer={
                <>
                    <FooterBtn onClick={onClose}>Cancel</FooterBtn>
                    <FooterBtn primary onClick={onClose}>
                        Create Instance
                    </FooterBtn>
                </>
            }
        >
            <Label>Instance name</Label>
            <InputRow value="Dragon Survival v2" placeholder="My New Instance"/>

            {/* Version + Java row */}
            <div className="grid grid-cols-2 gap-3.5 mt-4">
                <div>
                    <Label>Minecraft version</Label>
                    <VersionPicker/>
                </div>
                <div>
                    <Label>Java</Label>
                    <SelectRow value="Java 21 (bundled)" hint="Auto-matched to version"/>
                </div>
            </div>

            {/* Loader cards */}
            <Label className="mt-[18px]">Mod loader</Label>
            <div className="grid grid-cols-5 gap-2">
                {loaders.map((l) => {
                    const selected = l.id === SELECTED_LOADER;
                    return (
                        <div
                            key={l.id}
                            className="relative p-3 rounded-[10px] cursor-pointer border"
                            style={{
                                borderColor: selected ? l.color : "var(--line)",
                                borderWidth: selected ? 1.5 : 1,
                                background: selected
                                    ? `color-mix(in oklab, ${l.color} 10%, transparent)`
                                    : "rgba(255,255,255,0.02)",
                            }}
                        >
                            <div
                                className="w-7 h-7 rounded-[7px] mb-2"
                                style={{
                                    background: `linear-gradient(135deg, ${l.color}, color-mix(in oklab, ${l.color} 60%, #000))`,
                                    boxShadow: selected ? `0 0 12px ${l.color}80` : "none",
                                }}
                            />
                            <div className="text-xs font-semibold">{l.name}</div>
                            <div className="text-[0.625rem] text-ink-3 mt-0.5 leading-tight">
                                {l.desc}
                            </div>
                            {selected && (
                                <div
                                    className="absolute top-1.5 right-1.5"
                                    style={{color: l.color}}
                                >
                                    <I.check size={12}/>
                                </div>
                            )}
                        </div>
                    );
                })}
            </div>

            {/* Quick-start templates */}
            <div
                className="mt-[18px] p-3.5 rounded-xl"
                style={{
                    background: "rgba(34,255,132,0.04)",
                    border: "1px dashed color-mix(in oklab, var(--mc-green) 20%, transparent)",
                }}
            >
                <div className="flex items-center justify-between mb-2.5">
                    <div>
                        <div className="text-xs font-semibold text-mc-green">
                            Quick start from template
                        </div>
                        <div className="text-[0.6875rem] text-ink-3 mt-0.5">
                            Curated setups for common playstyles
                        </div>
                    </div>
                    <div className="text-[0.625rem] text-ink-3 font-mono">OPTIONAL</div>
                </div>
                <div className="flex gap-2">
                    {templates.map((t, i) => {
                        const IC = t.icon;
                        return (
                            <div
                                key={i}
                                className="flex-1 px-2.5 py-2.5 rounded-lg bg-[rgba(255,255,255,0.03)] border border-line flex flex-col gap-1.5 cursor-pointer hover:bg-[rgba(255,255,255,0.05)] transition-colors"
                            >
                                <IC size={14} className="text-ink-2"/>
                                <div className="text-[0.6875rem] font-semibold">{t.label}</div>
                                <div className="text-[0.625rem] text-ink-3">{t.mods} mods</div>
                            </div>
                        );
                    })}
                </div>
            </div>
        </ModalShell>
    );
}
