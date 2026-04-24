import WorldGridCard from "./WorldGridCard";
import type {World} from "./worldsData";

type Props = {
    list: World[];
};

// Grid view: 3-column layout of WorldGridCard tiles.
export default function WorldGrid({list}: Props) {
    return (
        <div className="grid grid-cols-3 gap-3.5">
            {list.map((w, i) => (
                <WorldGridCard key={i} world={w}/>
            ))}
        </div>
    );
}
