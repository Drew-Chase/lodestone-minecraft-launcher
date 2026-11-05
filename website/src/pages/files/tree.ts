import type {FileEntry} from "../../api/instances.ts";

export interface FolderNode {
    /** Full path from the root, with "/" between folders. The root is "". */
    path: string;
    name: string;
    children: FolderNode[];
}

export type SortKey = "name" | "size";
export type SortDir = "asc" | "desc";

/** The folder part of a file path. Files at the top level have "". */
export function folderOf(path: string): string {
    const slash = path.lastIndexOf("/");
    return slash < 0 ? "" : path.slice(0, slash);
}

export function nameOf(path: string): string {
    return path.slice(path.lastIndexOf("/") + 1);
}

/** Builds the folder tree from the file paths. Folders are sorted by name at every level. */
export function buildFolderTree(files: readonly FileEntry[]): FolderNode {
    const root: FolderNode = {path: "", name: "", children: []};
    const nodes = new Map<string, FolderNode>([["", root]]);

    for (const file of files) {
        let parent = root;
        let current = "";
        for (const segment of folderOf(file.path).split("/")) {
            if (segment === "") continue;
            current = current === "" ? segment : `${current}/${segment}`;
            let node = nodes.get(current);
            if (!node) {
                node = {path: current, name: segment, children: []};
                nodes.set(current, node);
                parent.children.push(node);
            }
            parent = node;
        }
    }

    const sortChildren = (node: FolderNode) => {
        node.children.sort((a, b) => a.name.localeCompare(b.name, undefined, {numeric: true}));
        node.children.forEach(sortChildren);
    };
    sortChildren(root);
    return root;
}

/** The files inside a folder, including those in its subfolders. The root is "", which holds every file. */
export function filesIn(files: readonly FileEntry[], folder: string): FileEntry[] {
    if (folder === "") return [...files];
    const prefix = `${folder}/`;
    return files.filter((file) => file.path.startsWith(prefix));
}

export function sortFiles(files: readonly FileEntry[], key: SortKey, dir: SortDir): FileEntry[] {
    const sign = dir === "asc" ? 1 : -1;
    return [...files].sort((a, b) => {
        const order = key === "size"
            ? a.size - b.size
            : nameOf(a.path).localeCompare(nameOf(b.path), undefined, {numeric: true});
        return order * sign || a.path.localeCompare(b.path);
    });
}
