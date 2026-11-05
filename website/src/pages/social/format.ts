/** A timestamp as the reader's local date and time. Unparseable values are shown as they came. */
export function formatWhen(value: string): string {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

/** "ABCD1234" becomes "ABCD-1234". Input that is not eight characters is returned without a change. */
export function formatFriendCode(code: string): string {
    const compact = code.replace(/-/g, "");
    if (compact.length !== 8) return code;
    return `${compact.slice(0, 4)}-${compact.slice(4)}`;
}

/** Copies text to the clipboard. Resolves false when the browser refuses. */
export async function copyText(text: string): Promise<boolean> {
    try {
        await navigator.clipboard.writeText(text);
        return true;
    } catch {
        return false;
    }
}
