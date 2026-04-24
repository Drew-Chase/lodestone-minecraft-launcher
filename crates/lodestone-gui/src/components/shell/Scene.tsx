import React from "react";

export type Biome = "forest" | "nether" | "end" | "ocean" | "desert" | "snow" | "cherry" | "mushroom";

type SceneProps = {
    biome?: Biome;
    seed?: number;
    style?: React.CSSProperties;
    children?: React.ReactNode;
};

// Synthetic voxel-ish scene cover — no external imagery, pure CSS + SVG.
// `biome` picks a palette class (see index.css), `seed` shifts the skyline silhouette.
export default function Scene({biome = "forest", seed = 0, style, children}: SceneProps) {
    // Deterministic PRNG
    const r = (i: number) => {
        const x = Math.sin((seed + 1) * 9301 + i * 49297) * 233280;
        return x - Math.floor(x);
    };

    const cols = 18;
    const heights = Array.from({length: cols}, (_, i) => 30 + r(i) * 55);
    const skyline = heights.map((h, i) => {
        const x = (i / (cols - 1)) * 100;
        return `${x}% ${100 - h}%`;
    });
    const hillPath = `polygon(0% 100%, ${skyline.join(", ")}, 100% 100%)`;

    const sunX = 15 + r(99) * 70;
    const sunY = 18 + r(100) * 20;

    const frontTerrainBg =
        biome === "nether"
            ? "linear-gradient(180deg, #5a1e18 0%, #2a0a08 100%)"
            : biome === "end"
                ? "linear-gradient(180deg, #3a2a5a 0%, #0f0a1a 100%)"
                : biome === "ocean"
                    ? "linear-gradient(180deg, #0e3566 0%, #02111f 100%)"
                    : biome === "desert"
                        ? "linear-gradient(180deg, #8a5e28 0%, #3e2912 100%)"
                        : biome === "snow"
                            ? "linear-gradient(180deg, #d8e5f5 0%, #4a6a8a 100%)"
                            : biome === "cherry"
                                ? "linear-gradient(180deg, #7c2a5a 0%, #2a0a1e 100%)"
                                : "linear-gradient(180deg, #2a5a34 0%, #0e2414 100%)";

    const grassColor =
        biome === "desert" ? "#c48b3a" : biome === "snow" ? "#e8f0fa" : biome === "cherry" ? "#d86bac" : "#3faa5a";

    const showGrass = biome !== "nether" && biome !== "end" && biome !== "ocean";

    return (
        <div
            className={`biome-${biome}`}
            style={{position: "relative", overflow: "hidden", width: "100%", height: "100%", ...style}}
        >
            {/* Sun / moon */}
            <div
                style={{
                    position: "absolute",
                    left: `${sunX}%`,
                    top: `${sunY}%`,
                    width: 60,
                    height: 60,
                    borderRadius: "50%",
                    background: biome === "nether" ? "#ffd98a" : biome === "end" ? "#fff" : "#fff2b8",
                    boxShadow:
                        biome === "nether" ? "0 0 60px #ff9b5a" : "0 0 60px rgba(255,242,184,0.5)",
                    opacity: 0.9,
                }}
            />
            {/* Back hills */}
            <div
                style={{
                    position: "absolute",
                    inset: 0,
                    clipPath:
                        "polygon(0% 100%, 0% 60%, 12% 58%, 22% 65%, 34% 52%, 48% 58%, 62% 50%, 76% 56%, 88% 48%, 100% 54%, 100% 100%)",
                    background: "linear-gradient(180deg, rgba(0,0,0,0.2) 0%, rgba(0,0,0,0.5) 100%)",
                    opacity: 0.55,
                }}
            />
            {/* Front terrain */}
            <div
                style={{
                    position: "absolute",
                    inset: 0,
                    clipPath: hillPath,
                    background: frontTerrainBg,
                }}
            />
            {/* Grass strip */}
            {showGrass && (
                <div
                    style={{
                        position: "absolute",
                        inset: 0,
                        clipPath: hillPath,
                        background: `linear-gradient(180deg,
              transparent 0%, transparent calc(100% - 100px),
              ${grassColor} calc(100% - 100px),
              ${grassColor} calc(100% - 96px),
              transparent calc(100% - 96px))`,
                    }}
                />
            )}
            {/* Pixel grid overlay for voxel feel */}
            <div
                style={{
                    position: "absolute",
                    inset: 0,
                    backgroundImage:
                        "linear-gradient(90deg, rgba(0,0,0,0.18) 1px, transparent 1px), linear-gradient(0deg, rgba(0,0,0,0.18) 1px, transparent 1px)",
                    backgroundSize: "16px 16px",
                    opacity: 0.35,
                    pointerEvents: "none",
                }}
            />
            {/* Vignette */}
            <div
                style={{
                    position: "absolute",
                    inset: 0,
                    background:
                        "radial-gradient(ellipse at center, transparent 40%, rgba(0,0,0,0.5) 100%)",
                    pointerEvents: "none",
                }}
            />
            {children}
        </div>
    );
}
