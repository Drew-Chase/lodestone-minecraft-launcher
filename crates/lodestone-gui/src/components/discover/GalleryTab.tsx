import {useCallback, useEffect, useRef, useState} from "react";
import {cardSurfaceStyle} from "../surfaces";
import {I} from "../shell/icons";

interface GalleryTabProps {
    images: string[];
    title?: string;
}

export default function GalleryTab({images, title}: GalleryTabProps) {
    const [selectedIndex, setSelectedIndex] = useState<number | null>(null);

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
            <div style={{display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(200px, 1fr))", gap: 12}}>
                {images.map((src, i) => (
                    <div
                        key={i}
                        className="border border-line rounded-xl overflow-hidden cursor-pointer transition-all hover:border-[rgba(34,255,132,0.3)]"
                        style={cardSurfaceStyle}
                        onClick={() => setSelectedIndex(i)}
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

            {selectedIndex !== null && (
                <GalleryLightbox
                    images={images}
                    currentIndex={selectedIndex}
                    onIndexChange={setSelectedIndex}
                    onClose={() => setSelectedIndex(null)}
                    title={title}
                />
            )}
        </>
    );
}

function GalleryLightbox({
    images,
    currentIndex,
    onIndexChange,
    onClose,
    title,
}: {
    images: string[];
    currentIndex: number;
    onIndexChange: (i: number) => void;
    onClose: () => void;
    title?: string;
}) {
    const hasPrev = currentIndex > 0;
    const hasNext = currentIndex < images.length - 1;
    const containerRef = useRef<HTMLDivElement>(null);
    const [imgSize, setImgSize] = useState<{width: number; height: number} | null>(null);

    // Compute the fitted image dimensions whenever the image or container changes.
    const computeSize = useCallback(() => {
        const container = containerRef.current;
        if (!container) return;
        const img = new Image();
        img.src = images[currentIndex];
        const update = () => {
            const cw = container.clientWidth;
            const ch = container.clientHeight;
            const nw = img.naturalWidth || 1;
            const nh = img.naturalHeight || 1;
            const scale = Math.min(cw / nw, ch / nh);
            setImgSize({width: nw * scale, height: nh * scale});
        };
        if (img.complete) {
            update();
        } else {
            img.onload = update;
        }
    }, [images, currentIndex]);

    useEffect(() => {
        computeSize();
        window.addEventListener("resize", computeSize);
        return () => window.removeEventListener("resize", computeSize);
    }, [computeSize]);

    const goPrev = useCallback(() => {
        if (hasPrev) onIndexChange(currentIndex - 1);
    }, [hasPrev, currentIndex, onIndexChange]);

    const goNext = useCallback(() => {
        if (hasNext) onIndexChange(currentIndex + 1);
    }, [hasNext, currentIndex, onIndexChange]);

    // Keyboard navigation
    useEffect(() => {
        const handler = (e: KeyboardEvent) => {
            if (e.key === "Escape") onClose();
            if (e.key === "ArrowLeft") goPrev();
            if (e.key === "ArrowRight") goNext();
        };
        window.addEventListener("keydown", handler);
        return () => window.removeEventListener("keydown", handler);
    }, [onClose, goPrev, goNext]);

    return (
        // Backdrop — clicking it dismisses the lightbox
        <div
            className="fixed inset-0 z-50 flex flex-col"
            style={{background: "rgba(0,0,0,0.9)", backdropFilter: "blur(12px)"}}
            onClick={onClose}
        >
            {/* Title + counter — positioned below the title bar / macOS stoplights */}
            <div
                className="absolute z-20 flex flex-col"
                style={{top: 40, left: 20}}
                onClick={e => e.stopPropagation()}
            >
                {title && (
                    <span style={{fontSize: 14, fontWeight: 700, color: "#fff"}}>
                        {title}
                    </span>
                )}
                <span style={{fontSize: 12, color: "var(--ink-3)"}}>
                    {currentIndex + 1} of {images.length}
                </span>
            </div>

            {/* Close button — positioned below the title bar, away from window controls */}
            <button
                onClick={(e) => { e.stopPropagation(); onClose(); }}
                className="absolute cursor-pointer flex items-center justify-center z-20"
                style={{
                    top: 40,
                    right: 20,
                    width: 36,
                    height: 36,
                    borderRadius: 8,
                    background: "rgba(0,0,0,0.5)",
                    border: "1px solid rgba(255,255,255,0.1)",
                    color: "var(--ink-2)",
                    backdropFilter: "blur(8px)",
                }}
            >
                <I.close size={18}/>
            </button>

            {/* Main image area — image scales to fill width or height */}
            <div
                ref={containerRef}
                className="flex-1 flex items-center justify-center min-h-0 relative px-16"
                style={{paddingTop: 70, paddingBottom: 8}}
            >
                {/* Previous button */}
                {hasPrev && (
                    <button
                        onClick={e => { e.stopPropagation(); goPrev(); }}
                        className="absolute left-4 cursor-pointer flex items-center justify-center z-10"
                        style={{
                            width: 44,
                            height: 44,
                            borderRadius: 999,
                            background: "rgba(0,0,0,0.5)",
                            border: "1px solid rgba(255,255,255,0.1)",
                            color: "#fff",
                            backdropFilter: "blur(8px)",
                        }}
                    >
                        <I.chevRight size={20} style={{transform: "rotate(180deg)"}}/>
                    </button>
                )}

                {/* Image — sized via JS so element bounds match rendered pixels,
                    fills available space, and clicks on dead space dismiss. */}
                <img
                    src={images[currentIndex]}
                    alt={`Screenshot ${currentIndex + 1}`}
                    onClick={e => e.stopPropagation()}
                    onLoad={computeSize}
                    style={{
                        width: imgSize?.width ?? "auto",
                        height: imgSize?.height ?? "auto",
                        maxWidth: "100%",
                        maxHeight: "100%",
                        borderRadius: 8,
                    }}
                />

                {/* Next button */}
                {hasNext && (
                    <button
                        onClick={e => { e.stopPropagation(); goNext(); }}
                        className="absolute right-4 cursor-pointer flex items-center justify-center z-10"
                        style={{
                            width: 44,
                            height: 44,
                            borderRadius: 999,
                            background: "rgba(0,0,0,0.5)",
                            border: "1px solid rgba(255,255,255,0.1)",
                            color: "#fff",
                            backdropFilter: "blur(8px)",
                        }}
                    >
                        <I.chevRight size={20}/>
                    </button>
                )}
            </div>

            {/* Thumbnail strip */}
            <div
                className="flex-shrink-0 flex items-center justify-center gap-2 overflow-x-auto"
                style={{padding: "12px 20px"}}
                onClick={e => e.stopPropagation()}
            >
                {images.map((src, i) => (
                    <div
                        key={i}
                        className="flex-shrink-0 cursor-pointer rounded-lg overflow-hidden transition-all"
                        style={{
                            width: 64,
                            height: 44,
                            border: i === currentIndex
                                ? "2px solid var(--mc-green)"
                                : "2px solid transparent",
                            opacity: i === currentIndex ? 1 : 0.5,
                        }}
                        onClick={() => onIndexChange(i)}
                    >
                        <img
                            src={src}
                            alt={`Thumbnail ${i + 1}`}
                            className="w-full h-full object-cover"
                            loading="lazy"
                        />
                    </div>
                ))}
            </div>
        </div>
    );
}
