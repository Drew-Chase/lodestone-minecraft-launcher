import InstanceCompactCard from "./InstanceCompactCard";
import type {Instance} from "./instances";

type Props = {
    list: Instance[];
    onDeleteRequest: (inst: Instance) => void;
};

// Compact view: 4-column layout of InstanceCompactCard tiles.
export default function InstanceCompact({list, onDeleteRequest}: Props) {
    return (
        <div style={{display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 10}}>
            {list.map((inst, i) => (
                <InstanceCompactCard key={i} instance={inst} onDeleteRequest={onDeleteRequest}/>
            ))}
        </div>
    );
}
