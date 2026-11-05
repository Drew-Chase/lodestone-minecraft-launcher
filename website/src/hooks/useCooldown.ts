import {useEffect, useState} from "react";

/**
 * A cooldown for a button. `start()` begins a cooldown of `seconds` from the moment it is called, and `remaining` counts
 * it down to 0 once a second.
 */
export function useCooldown(seconds: number): { remaining: number; start: () => void } {
    const [until, setUntil] = useState(0);
    const [now, setNow] = useState(0);

    useEffect(() => {
        if (until === 0) return;
        let timer: ReturnType<typeof setInterval> | undefined;
        const tick = () => {
            const current = Date.now();
            setNow(current);
            if (current >= until && timer !== undefined) clearInterval(timer);
        };
        tick();
        timer = setInterval(tick, 1000);
        return () => clearInterval(timer);
    }, [until]);

    const remaining = until === 0 ? 0 : Math.max(0, Math.ceil((until - now) / 1000));

    return {
        remaining,
        start: () => {
            const current = Date.now();
            setNow(current);
            setUntil(current + seconds * 1000);
        },
    };
}
