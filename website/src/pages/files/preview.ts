/** Text files larger than this are not fetched for a preview. */
export const TEXT_PREVIEW_LIMIT = 256 * 1024;

const IMAGE_EXTENSIONS: ReadonlySet<string> = new Set(["png", "jpg", "jpeg", "gif", "webp"]);

const TEXT_EXTENSIONS: ReadonlySet<string> = new Set([
    "txt", "json", "toml", "cfg", "properties", "yml", "yaml", "md", "log", "lang", "mcmeta", "json5",
]);

export type PreviewKind = "image" | "text" | "none";

export function previewKind(path: string): PreviewKind {
    const name = path.slice(path.lastIndexOf("/") + 1);
    const dot = name.lastIndexOf(".");
    if (dot < 0) return "none";
    const extension = name.slice(dot + 1).toLowerCase();
    if (IMAGE_EXTENSIONS.has(extension)) return "image";
    if (TEXT_EXTENSIONS.has(extension)) return "text";
    return "none";
}
