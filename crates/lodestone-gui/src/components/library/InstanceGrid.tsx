import InstanceGridCard from "./InstanceGridCard";
import type {Instance} from "./instances";

type Props = {
    list: Instance[];
    onDeleteRequest: (inst: Instance) => void;
};

// Grid view: 3-column layout of InstanceGridCard tiles.
export default function InstanceGrid({list, onDeleteRequest}: Props) {
    return (
        <div style={{display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))", gap: 14}}>
            {list.map((inst, i) => (
                <InstanceGridCard key={i} instance={inst} onDeleteRequest={onDeleteRequest}/>
            ))}
        </div>
    );
}
