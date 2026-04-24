import type {SortKey} from "../../types/content";

const options: {key: SortKey; label: string}[] = [
    {key: "relevance", label: "Relevance"},
    {key: "downloads", label: "Downloads"},
    {key: "follows", label: "Follows"},
    {key: "updated", label: "Updated"},
    {key: "latest", label: "Newest"},
];

export default function SortSelect({value, onChange}: {value: SortKey; onChange: (v: SortKey) => void}) {
    return (
        <select
            value={value}
            onChange={e => onChange(e.target.value as SortKey)}
            className="select"
            style={{
                background: "rgba(0,0,0,0.3)",
                border: "1px solid var(--line)",
                borderRadius: 6,
                padding: "7px 10px",
                fontSize: 12,
                color: "var(--ink-1)",
                fontWeight: 600,
                cursor: "pointer",
            }}
        >
            {options.map(o => (
                <option key={o.key} value={o.key}>{o.label}</option>
            ))}
        </select>
    );
}
