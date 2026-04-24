import type {ContentTypeKey, SourceKey} from "../../types/content";

interface SourceTabsProps {
    active: SourceKey;
    contentType: ContentTypeKey;
    onChange: (key: SourceKey) => void;
}

type SourceDef = {key: SourceKey; label: string; disabled?: boolean};

const baseSources: SourceDef[] = [
    {key: "all", label: "All providers"},
    {key: "modrinth", label: "Modrinth"},
    {key: "curseforge", label: "CurseForge"},
];

const modpackExtras: SourceDef[] = [
    {key: "all", label: "ATLauncher", disabled: true},
    {key: "all", label: "FTB", disabled: true},
    {key: "all", label: "Technic", disabled: true},
];

export default function SourceTabs({active, contentType, onChange}: SourceTabsProps) {
    const sources = contentType === "modpack"
        ? [...baseSources, ...modpackExtras]
        : baseSources;

    return (
        <div className="flex gap-1">
            {sources.map((s, i) => {
                const isActive = !s.disabled && s.key === active;
                return (
                    <button
                        key={`${s.label}-${i}`}
                        onClick={() => !s.disabled && onChange(s.key)}
                        disabled={s.disabled}
                        className="cursor-pointer"
                        style={{
                            padding: "7px 12px",
                            fontSize: 12,
                            fontWeight: 600,
                            borderRadius: 8,
                            border: isActive
                                ? "1px solid rgba(34,255,132,0.4)"
                                : "1px solid transparent",
                            background: isActive
                                ? "rgba(34,255,132,0.08)"
                                : "transparent",
                            color: s.disabled
                                ? "var(--ink-4)"
                                : isActive ? "#fff" : "var(--ink-2)",
                            transition: "all 0.12s",
                            opacity: s.disabled ? 0.5 : 1,
                        }}
                    >
                        {s.label}
                    </button>
                );
            })}
        </div>
    );
}
