import {useState} from "react";
import {cardSurfaceStyle} from "../surfaces";
import {I} from "../shell/icons";

export default function GalleryTab({images}: {images: string[]}) {
    const [selected, setSelected] = useState<string | null>(null);

    if (images.length === 0) {
        return (
            <div
                className="border border-line rounded-xl flex flex-col items-center justify-center gap-3"
                style={{...cardSurfaceStyle, padding: 48}}
            >
                <I.image size={32} style={{color: "var(--ink-4)"}}/>
                <span style={{fontSize: 13, color: "var(--ink-3)"}}>No screenshots available</span>
            </div>
        );
    }

    return (
        <>
            <div style={{display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 12}}>
                {images.map((src, i) => (
                    <div
                        key={i}
                        className="border border-line rounded-xl overflow-hidden cursor-pointer transition-all hover:border-[rgba(34,255,132,0.3)]"
                        style={cardSurfaceStyle}
                        onClick={() => setSelected(src)}
                    >
                        <img
                            src={src}
                            alt={`Screenshot ${i + 1}`}
                            className="w-full object-cover"
                            style={{height: 160}}
                            loading="lazy"
                        />
                    </div>
                ))}
            </div>

            {/* Lightbox */}
            {selected && (
                <div
                    className="fixed inset-0 flex items-center justify-center z-50 cursor-pointer"
                    style={{background: "rgba(0,0,0,0.8)", backdropFilter: "blur(8px)"}}
                    onClick={() => setSelected(null)}
                >
                    <img
                        src={selected}
                        alt="Full size"
                        style={{maxWidth: "90vw", maxHeight: "90vh", borderRadius: 12}}
                    />
                </div>
            )}
        </>
    );
}
