import InstanceCompactCard from "./InstanceCompactCard";
import type {Instance} from "./instances";

type Props = {
    list: Instance[];
};

// Compact view: 4-column layout of InstanceCompactCard tiles.
export default function InstanceCompact({list}: Props) {
    return (
        <div style={{display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 10}}>
            {list.map((inst, i) => (
                <InstanceCompactCard key={i} instance={inst}/>
            ))}
        </div>
    );
}
