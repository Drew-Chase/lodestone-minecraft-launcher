import {I} from "../shell/icons";
import type {ViewMode} from "../../types/content";

const modes: {key: ViewMode; icon: keyof typeof I}[] = [
    {key: "grid", icon: "grid"},
    {key: "compact", icon: "list"},
    {key: "table", icon: "table"},
];

export default function ViewModeToggle({value, onChange}: {value: ViewMode; onChange: (v: ViewMode) => void}) {
    return (
        <div className="flex gap-0 border border-line rounded-lg overflow-hidden">
            {modes.map(m => {
                const Icon = I[m.icon];
                const active = m.key === value;
                return (
                    <button
                        key={m.key}
                        onClick={() => onChange(m.key)}
                        className="cursor-pointer"
                        style={{
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            width: 32,
                            height: 30,
                            background: active ? "rgba(34,255,132,0.1)" : "transparent",
                            color: active ? "var(--mc-green)" : "var(--ink-3)",
                            border: "none",
                            transition: "all 0.12s",
                        }}
                    >
                        <Icon size={14}/>
                    </button>
                );
            })}
        </div>
    );
}
