import InstanceGridCard from "./InstanceGridCard";
import type {Instance} from "./instances";

type Props = {
    list: Instance[];
};

// Grid view: 3-column layout of InstanceGridCard tiles.
export default function InstanceGrid({list}: Props) {
    return (
        <div className="grid grid-cols-3 gap-4">
            {list.map((inst, i) => (
                <InstanceGridCard key={i} instance={inst}/>
            ))}
        </div>
    );
}
