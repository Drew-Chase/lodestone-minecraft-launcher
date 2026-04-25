import type {PlatformId} from "../../types/content";

const styles: Record<string, React.CSSProperties> = {
    Modrinth: {
        background: "rgba(34,255,132,0.12)",
        color: "var(--mc-green)",
        border: "1px solid rgba(34,255,132,0.3)",
    },
    CurseForge: {
        background: "rgba(255,107,61,0.14)",
        color: "#ff8c5a",
        border: "1px solid rgba(255,107,61,0.3)",
    },
};

export default function SourceBadge({platform}: {platform: PlatformId}) {
    const s = styles[platform];
    if (!s) return null;
    return (
        <span
            style={{
                ...s,
                padding: "3px 7px",
                borderRadius: 999,
                fontSize: 9,
                fontFamily: "var(--mono)",
                fontWeight: 700,
                letterSpacing: 0.5,
                textTransform: "uppercase",
                textShadow: "0 1px 3px rgba(0,0,0,0.6)",
                boxShadow: "0 1px 4px rgba(0,0,0,0.4)",
            }}
        >
            {platform}
        </span>
    );
}
