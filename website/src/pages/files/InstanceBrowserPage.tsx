import {useEffect, useMemo, useState} from "react";
import {Link, useParams} from "react-router";
import "../../css/files.css";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {getInstance, getVersion, listVersions} from "../../api/instances.ts";
import type {FileEntry, Instance, Version, VersionDetail} from "../../api/instances.ts";
import {messageFor} from "../../api/errors.ts";
import {formatWhen} from "../social/format.ts";
import FilePreview from "./FilePreview.tsx";
import FileTable from "./FileTable.tsx";
import FolderTree from "./FolderTree.tsx";
import RoleBadge from "./RoleBadge.tsx";
import {fileCountLabel, formatSize} from "./format.ts";
import {buildFolderTree, filesIn, sortFiles} from "./tree.ts";
import type {SortDir, SortKey} from "./tree.ts";

// Presigned addresses last 15 minutes, so the open version is fetched again before they run out.
const REFRESH_MS = 12 * 60 * 1000;

const NOT_FOUND_MESSAGES: Readonly<Record<string, string>> = {
    not_found: "That instance was not found.",
};

// Browses one instance's files. The instance and its versions load first, then the newest version's files.
export default function InstanceBrowserPage() {
    const params = useParams();
    const id = Number(params.id);
    const validId = Number.isSafeInteger(id) && id > 0;

    const [attempt, setAttempt] = useState(0);
    const [instance, setInstance] = useState<Instance | null>(null);
    const [versions, setVersions] = useState<Version[] | null>(null);
    const [loadFailure, setLoadFailure] = useState<string | null>(null);
    // null means the newest version. A pick from the select replaces it.
    const [picked, setPicked] = useState<number | null>(null);
    const [detail, setDetail] = useState<VersionDetail | null>(null);
    const [detailFailure, setDetailFailure] = useState<string | null>(null);

    useEffect(() => {
        if (!validId) return;
        let active = true;
        setInstance(null);
        setVersions(null);
        setLoadFailure(null);
        setPicked(null);
        getInstance(id)
            .then(async (found) => {
                const list = await listVersions(id);
                if (active) {
                    setInstance(found);
                    setVersions(list);
                }
            })
            .catch((err: unknown) => {
                if (active) setLoadFailure(messageFor(err, NOT_FOUND_MESSAGES));
            });
        return () => {
            active = false;
        };
    }, [id, validId, attempt]);

    const selectedId = picked ?? versions?.[0]?.id ?? null;

    useEffect(() => {
        if (!validId || selectedId === null) return;
        let active = true;
        const load = () => {
            getVersion(id, selectedId).then((next) => {
                if (!active) return;
                setDetail(next);
                setDetailFailure(null);
            }, (err: unknown) => {
                if (active) setDetailFailure(messageFor(err));
            });
        };
        setDetail(null);
        setDetailFailure(null);
        load();
        const timer = window.setInterval(load, REFRESH_MS);
        return () => {
            active = false;
            window.clearInterval(timer);
        };
    }, [id, validId, selectedId]);

    if (!validId) {
        return (
            <div className="fs-page">
                <h1 className="acct-title">Files</h1>
                <FormAlert message="That instance was not found."/>
            </div>
        );
    }

    if (loadFailure) {
        return (
            <div className="fs-page">
                <Link className="fs-back" to="/dashboard/files">Back to files</Link>
                <h1 className="acct-title">Files</h1>
                <FormAlert message={loadFailure}/>
                <div className="acct-actions">
                    <button className="btn btn-primary acct-submit" type="button" onClick={() => setAttempt((n) => n + 1)}>
                        Try again
                    </button>
                </div>
            </div>
        );
    }

    if (instance === null || versions === null) {
        return (
            <div className="fs-page">
                <h1 className="acct-title">Files</h1>
                <p className="acct-status" role="status">Loading files...</p>
            </div>
        );
    }

    return (
        <div className="fs-page fs-page-wide">
            <Link className="fs-back" to="/dashboard/files">Back to files</Link>
            <div className="fs-header">
                <h1 className="acct-title">{instance.name}</h1>
                <RoleBadge role={instance.role}/>
            </div>

            {versions.length === 0 ? (
                <p className="acct-body">This instance has no pushed versions yet.</p>
            ) : (
                <>
                    <div className="fs-version-row">
                        <div className="acct-field fs-version-field">
                            <label className="acct-label" htmlFor="fs-version">Version</label>
                            <select
                                id="fs-version"
                                className="acct-input fs-select"
                                value={selectedId ?? ""}
                                onChange={(e) => setPicked(Number(e.target.value))}
                            >
                                {versions.map((version) => (
                                    <option key={version.id} value={version.id}>
                                        {formatWhen(version.created_at)} · {fileCountLabel(version.file_count)}
                                    </option>
                                ))}
                            </select>
                        </div>
                        {detail && (
                            <p className="fs-meta">
                                <span>Pushed {formatWhen(detail.version.created_at)}</span>
                                <span>{formatSize(detail.version.total_bytes)}</span>
                            </p>
                        )}
                    </div>

                    <FormAlert message={detailFailure}/>
                    {detail === null && detailFailure === null && (
                        <p className="acct-status" role="status">Loading files...</p>
                    )}
                    {detail && <FileBrowser key={detail.version.id} files={detail.files}/>}
                </>
            )}
        </div>
    );
}

// The folder tree, the table and the preview for one version. Keyed by version, so a new version starts clean.
function FileBrowser({files}: { files: FileEntry[] }) {
    const [folder, setFolder] = useState("");
    // Folders are open unless they are in this set, so folders that appear later open by default.
    const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(() => new Set());
    const [sortKey, setSortKey] = useState<SortKey>("name");
    const [sortDir, setSortDir] = useState<SortDir>("asc");
    const [selectedPath, setSelectedPath] = useState<string | null>(null);

    const tree = useMemo(() => buildFolderTree(files), [files]);
    const listing = useMemo(
        () => sortFiles(filesIn(files, folder), sortKey, sortDir),
        [files, folder, sortKey, sortDir],
    );
    const selected = files.find((file) => file.path === selectedPath) ?? null;

    function onSort(key: SortKey) {
        if (key === sortKey) {
            setSortDir((dir) => (dir === "asc" ? "desc" : "asc"));
        } else {
            setSortKey(key);
            setSortDir("asc");
        }
    }

    function onToggle(path: string) {
        setCollapsed((current) => {
            const next = new Set(current);
            if (next.has(path)) next.delete(path);
            else next.add(path);
            return next;
        });
    }

    return (
        <>
            <div className="fs-browser">
                <FolderTree
                    root={tree}
                    selected={folder}
                    collapsed={collapsed}
                    onSelect={setFolder}
                    onToggle={onToggle}
                />
                <div className="fs-listing">
                    {files.length === 0 ? (
                        <p className="acct-body">This version has no files.</p>
                    ) : listing.length === 0 ? (
                        <p className="acct-body">This folder is empty.</p>
                    ) : (
                        <FileTable
                            files={listing}
                            selectedPath={selectedPath}
                            sortKey={sortKey}
                            sortDir={sortDir}
                            onSort={onSort}
                            onSelect={setSelectedPath}
                        />
                    )}
                </div>
            </div>
            <FilePreview file={selected}/>
        </>
    );
}
