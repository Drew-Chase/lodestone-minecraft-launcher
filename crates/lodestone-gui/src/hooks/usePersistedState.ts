import {useCallback, useEffect, useState} from "react";

// React state hook that mirrors its value to localStorage under the given key.
// On mount (and whenever `key` changes) the hook re-reads from storage so
// per-entity keys (e.g. "instanceDetail.tab.<slug>") naturally swap between
// entities without losing per-entity memory. Writes happen synchronously from
// the returned setter, not from a state effect, which avoids the "write the
// old value to the new key" race when the key changes.
export function usePersistedState<T>(
    key: string,
    initial: T,
): [T, (v: T) => void] {
    const [state, setState] = useState<T>(() => readStored(key, initial));

    // Re-read when the key changes (e.g. navigating between instances).
    useEffect(() => {
        setState(readStored(key, initial));
        // `initial` intentionally omitted — we don't want literal-object initials
        // (like the strings used at call sites) to retrigger this effect.
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [key]);

    const update = useCallback(
        (v: T) => {
            setState(v);
            try {
                window.localStorage.setItem(key, JSON.stringify(v));
            } catch {
                // Ignore — private-mode Safari and disabled storage drop writes.
            }
        },
        [key],
    );

    return [state, update];
}

function readStored<T>(key: string, fallback: T): T {
    try {
        const raw = window.localStorage.getItem(key);
        if (raw === null) return fallback;
        return JSON.parse(raw) as T;
    } catch {
        return fallback;
    }
}
