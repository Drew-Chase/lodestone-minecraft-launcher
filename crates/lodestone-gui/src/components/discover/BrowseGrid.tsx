import type {ContentItem} from "../../types/content";
import BrowseCard from "./BrowseCard";

export default function BrowseGrid({items}: {items: ContentItem[]}) {
    return (
        <div style={{display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 14}}>
            {items.map((item, i) => (
                <BrowseCard key={`${item.id}-${i}`} item={item}/>
            ))}
        </div>
    );
}
