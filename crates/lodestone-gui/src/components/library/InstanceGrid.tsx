import InstanceGridCard from "./InstanceGridCard";
import type {Instance} from "./instances";

type Props = {
    list: Instance[];
};

// Grid view: 3-column layout of InstanceGridCard tiles.
export default function InstanceGrid({list}: Props) {
    return (
        <div className="grid gap-4" style={{gridTemplateColumns: "repeat(auto-fit, minmax(300px, 1fr))"}}>
            {list.map((inst, i) => (
                <InstanceGridCard key={i} instance={inst}/>
            ))}
        </div>
    );
}
