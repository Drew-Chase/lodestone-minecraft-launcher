import InstanceCompactCard from "./InstanceCompactCard";
import type {Instance} from "./instances";

type Props = {
    list: Instance[];
};

// Compact view: 4-column layout of InstanceCompactCard tiles.
export default function InstanceCompact({list}: Props) {
    return (
        <div className="grid gap-2" style={{gridTemplateColumns: "repeat(auto-fit, minmax(280px, 1fr))"}}>
            {list.map((inst, i) => (
                <InstanceCompactCard key={i} instance={inst}/>
            ))}
        </div>
    );
}
