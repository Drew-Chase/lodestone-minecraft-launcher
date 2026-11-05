import type {FileEntry} from "../../api/instances.ts";
import {formatModified, formatSize, shortHash} from "./format.ts";
import {folderOf, nameOf} from "./tree.ts";
import type {SortDir, SortKey} from "./tree.ts";

interface FileTableProps {
    files: FileEntry[];
    selectedPath: string | null;
    sortKey: SortKey;
    sortDir: SortDir;
    onSort: (key: SortKey) => void;
    onSelect: (path: string) => void;
}

export default function FileTable({files, selectedPath, sortKey, sortDir, onSort, onSelect}: FileTableProps) {
    return (
        <div className="fs-table-wrap">
            <table className="fs-table">
                <thead>
                <tr>
                    <SortHeader label="Name" sortKey="name" activeKey={sortKey} dir={sortDir} onSort={onSort}/>
                    <th scope="col">Folder</th>
                    <SortHeader label="Size" sortKey="size" activeKey={sortKey} dir={sortDir} onSort={onSort}/>
                    <th scope="col">Modified</th>
                    <th scope="col">SHA-256</th>
                </tr>
                </thead>
                <tbody>
                {files.map((file) => {
                    const selected = file.path === selectedPath;
                    const folder = folderOf(file.path);
                    return (
                        <tr key={file.path} className={selected ? "is-selected" : undefined} onClick={() => onSelect(file.path)}>
                            <td>
                                <button
                                    type="button"
                                    className="fs-file-name"
                                    aria-pressed={selected}
                                    title={file.path}
                                >
                                    {nameOf(file.path)}
                                </button>
                            </td>
                            <td className="fs-cell-muted">{folder === "" ? "/" : folder}</td>
                            <td>{formatSize(file.size)}</td>
                            <td>{formatModified(file.mtime_ms)}</td>
                            <td><code className="fs-hash" title={file.sha256}>{shortHash(file.sha256)}</code></td>
                        </tr>
                    );
                })}
                </tbody>
            </table>
        </div>
    );
}

interface SortHeaderProps {
    label: string;
    sortKey: SortKey;
    activeKey: SortKey;
    dir: SortDir;
    onSort: (key: SortKey) => void;
}

function SortHeader({label, sortKey, activeKey, dir, onSort}: SortHeaderProps) {
    const active = sortKey === activeKey;
    const arrow = active ? (dir === "asc" ? " ▲" : " ▼") : "";
    return (
        <th scope="col" aria-sort={active ? (dir === "asc" ? "ascending" : "descending") : "none"}>
            <button type="button" className="fs-sort" onClick={() => onSort(sortKey)}>
                {label}{arrow}
            </button>
        </th>
    );
}
