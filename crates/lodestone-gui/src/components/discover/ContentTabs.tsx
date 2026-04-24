import type {ContentTypeKey} from "../../types/content";

const tabs: {key: ContentTypeKey; label: string}[] = [
    {key: "modpack", label: "Modpacks"},
    {key: "mod", label: "Mods"},
    {key: "resourcepack", label: "Resource Packs"},
    {key: "shaderpack", label: "Shaders"},
    {key: "datapack", label: "Data Packs"},
    {key: "world", label: "Worlds"},
];

interface ContentTabsProps {
    active: ContentTypeKey;
    onChange: (key: ContentTypeKey) => void;
}

export default function ContentTabs({active, onChange}: ContentTabsProps) {
    return (
        <div className="flex gap-0 border-b border-line" style={{padding: "0 28px"}}>
            {tabs.map(t => {
                const isActive = t.key === active;
                return (
                    <button
                        key={t.key}
                        onClick={() => onChange(t.key)}
                        className="relative bg-transparent cursor-pointer"
                        style={{
                            padding: "10px 4px",
                            marginRight: 14,
                            fontSize: 13,
                            fontWeight: isActive ? 700 : 500,
                            color: isActive ? "#fff" : "var(--ink-2)",
                            border: "none",
                            transition: "color 0.12s",
                        }}
                    >
                        {t.label}
                        {isActive && (
                            <span
                                style={{
                                    position: "absolute",
                                    bottom: -1,
                                    left: 0,
                                    right: 0,
                                    height: 2,
                                    borderRadius: 2,
                                    background: "var(--mc-green)",
                                    boxShadow: "0 0 8px var(--mc-green), 0 0 14px var(--mc-green-glow, rgba(34,255,132,0.3))",
                                }}
                            />
                        )}
                    </button>
                );
            })}
        </div>
    );
}
