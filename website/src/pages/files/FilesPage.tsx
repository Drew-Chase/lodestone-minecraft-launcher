import {useEffect, useState} from "react";
import {Link} from "react-router";
import "../../css/files.css";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {fetchStorage} from "../../api/storage.ts";
import type {Storage} from "../../api/storage.ts";
import {listInstances} from "../../api/instances.ts";
import type {Instance} from "../../api/instances.ts";
import {messageFor} from "../../api/errors.ts";
import {formatWhen} from "../social/format.ts";
import ManageInstance from "./ManageInstance.tsx";
import NewInstanceForm from "./NewInstanceForm.tsx";
import RoleBadge from "./RoleBadge.tsx";
import StorageMeter from "./StorageMeter.tsx";
import {fileCountLabel} from "./format.ts";

// The files list: storage, the instances the account owns or was shared, and the owner's controls for one of them.
export default function FilesPage() {
    const [storage, setStorage] = useState<Storage | null>(null);
    const [storageFailure, setStorageFailure] = useState<string | null>(null);
    const [instances, setInstances] = useState<Instance[] | null>(null);
    const [listFailure, setListFailure] = useState<string | null>(null);
    const [managedId, setManagedId] = useState<number | null>(null);
    const [attempt, setAttempt] = useState(0);

    async function reloadStorage(): Promise<void> {
        try {
            setStorage(await fetchStorage());
            setStorageFailure(null);
        } catch (err) {
            setStorageFailure(messageFor(err));
        }
    }

    async function reloadInstances(): Promise<void> {
        try {
            setInstances(await listInstances());
            setListFailure(null);
        } catch (err) {
            setListFailure(messageFor(err));
        }
    }

    async function refreshAll(): Promise<void> {
        await Promise.all([reloadStorage(), reloadInstances()]);
    }

    useEffect(() => {
        void refreshAll();
    }, [attempt]);

    // Only an owned instance can be managed. A share that was revoked since the last load drops out here.
    const managed = instances?.find((instance) => instance.id === managedId && instance.role === "owner") ?? null;

    return (
        <div className="fs-page">
            <h1 className="acct-title">Files</h1>

            <StorageMeter storage={storage} failure={storageFailure}/>

            <NewInstanceForm onCreated={refreshAll}/>

            <section className="acct-section" aria-labelledby="fs-instances-title">
                <h2 className="acct-subtitle" id="fs-instances-title">Instances</h2>
                <FormAlert message={listFailure}/>
                {instances === null && listFailure === null && (
                    <p className="acct-status" role="status">Loading instances...</p>
                )}
                {instances !== null && instances.length === 0 && (
                    <p className="acct-body">No instances yet. Create one above.</p>
                )}
                {instances !== null && instances.length > 0 && (
                    <InstanceList
                        instances={instances}
                        managedId={managed?.id ?? null}
                        onManage={(id) => setManagedId((current) => (current === id ? null : id))}
                    />
                )}
                {instances === null && listFailure !== null && (
                    <div className="acct-actions">
                        <button className="btn btn-primary acct-submit" type="button"
                                onClick={() => setAttempt((n) => n + 1)}>
                            Try again
                        </button>
                    </div>
                )}
            </section>

            {managed && (
                <ManageInstance
                    key={managed.id}
                    instance={managed}
                    onDeleted={async () => {
                        setManagedId(null);
                        await refreshAll();
                    }}
                />
            )}
        </div>
    );
}

interface InstanceListProps {
    instances: Instance[];
    managedId: number | null;
    onManage: (id: number) => void;
}

function InstanceList({instances, managedId, onManage}: InstanceListProps) {
    return (
        <ul className="soc-list">
            {instances.map((instance) => {
                const latest = instance.latest_version;
                const ownerName = instance.owner.username ?? "No username";
                const managing = managedId === instance.id;
                return (
                    <li className="soc-row fs-row" key={instance.id}>
                        <div className="fs-row-info">
                            <div className="fs-row-title">
                                <Link className="soc-name fs-row-name" to={`/dashboard/files/${instance.id}`}>
                                    {instance.name}
                                </Link>
                                <RoleBadge role={instance.role}/>
                            </div>
                            <p className="fs-meta">
                                <span>Owner {ownerName}</span>
                                <span>Last push {latest ? formatWhen(latest.created_at) : "never"}</span>
                                <span>{fileCountLabel(latest ? latest.file_count : 0)}</span>
                            </p>
                        </div>
                        {instance.role === "owner" && (
                            <div className="soc-row-actions">
                                <button
                                    className="btn btn-ghost soc-action"
                                    type="button"
                                    aria-pressed={managing}
                                    aria-label={`${managing ? "Close" : "Manage"} ${instance.name}`}
                                    onClick={() => onManage(instance.id)}
                                >
                                    {managing ? "Close" : "Manage"}
                                </button>
                            </div>
                        )}
                    </li>
                );
            })}
        </ul>
    );
}
