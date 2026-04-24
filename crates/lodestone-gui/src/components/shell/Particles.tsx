type ParticlesProps = {
    count?: number;
    color?: string;
};

// Drifts `count` green dots upwards with the `drift` keyframe from index.css.
export default function Particles({count = 18, color = "var(--mc-green)"}: ParticlesProps) {
    const particles = Array.from({length: count}, (_, i) => i);
    return (
        <div style={{position: "absolute", inset: 0, pointerEvents: "none", overflow: "hidden"}}>
            {particles.map((i) => {
                const delay = (i * 0.37) % 12;
                const dur = 8 + ((i * 0.7) % 6);
                const left = (i * 17.3) % 100;
                const size = 1 + (i % 3);
                return (
                    <div
                        key={i}
                        style={{
                            position: "absolute",
                            left: `${left}%`,
                            bottom: -10,
                            width: size,
                            height: size,
                            borderRadius: "50%",
                            background: color,
                            boxShadow: `0 0 ${4 + size}px ${color}`,
                            opacity: 0.6,
                            animation: `drift ${dur}s linear ${delay}s infinite`,
                        }}
                    />
                );
            })}
        </div>
    );
}
