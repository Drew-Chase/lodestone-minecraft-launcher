import type {Difficulty, Gamemode} from "./worldsData";

type Props = {
    gamemode: Gamemode;
    difficulty: Difficulty;
};

// Small inline indicator used on every world tile: colored dot (red / cyan /
// green depending on gamemode) plus uppercase `GAMEMODE · DIFFICULTY` mono text.
export default function WorldMode({gamemode, difficulty}: Props) {
    const color =
        gamemode === "Hardcore"
            ? "#ff5e5e"
            : gamemode === "Creative"
                ? "var(--cyan)"
                : "var(--mc-green)";

    return (
        <span className="inline-flex items-center gap-1.5 font-mono font-bold text-[0.625rem] tracking-[0.3px]">
      <span
          className="w-1.5 h-1.5 rounded-full"
          style={{background: color, boxShadow: `0 0 6px ${color}`}}
      />
            <span className="text-ink-2">{gamemode.toUpperCase()}</span>
            <span className="text-ink-4">·</span>
            <span className="text-ink-3">{difficulty.toUpperCase()}</span>
    </span>
    );
}
