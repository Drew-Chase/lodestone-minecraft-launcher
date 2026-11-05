import {useState} from "react";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {deleteInstance} from "../../api/instances.ts";
import type {Instance} from "../../api/instances.ts";
import {messageFor} from "../../api/errors.ts";
import SharingPanel from "./SharingPanel.tsx";

// The owner's controls for one instance: sharing, then deleting it.
export default function ManageInstance({instance, onDeleted}: {
    instance: Instance;
    onDeleted: () => Promise<void>;
}) {
    return (
        <div className="fs-manage">
            <h2 className="acct-subtitle">Manage {instance.name}</h2>
            <SharingPanel instanceId={instance.id}/>
            <DeleteSection instance={instance} onDeleted={onDeleted}/>
        </div>
    );
}

function DeleteSection({instance, onDeleted}: { instance: Instance; onDeleted: () => Promise<void> }) {
    const [confirming, setConfirming] = useState(false);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);

    async function onDelete() {
        if (busy) return;
        setBusy(true);
        setError(null);
        try {
            await deleteInstance(instance.id);
        } catch (err) {
            setError(messageFor(err, {not_found: "That instance is already gone."}));
            setBusy(false);
            setConfirming(false);
            return;
        }
        await onDeleted();
    }

    return (
        <section className="acct-section" aria-labelledby="fs-delete-title">
            <h2 className="acct-subtitle" id="fs-delete-title">Delete instance</h2>
            <p className="acct-body">Deleting removes this instance and all of its versions. This cannot be undone.</p>
            {confirming ? (
                <div className="acct-actions" role="group" aria-label="Confirm delete">
                    <p className="acct-body">Delete {instance.name}?</p>
                    <div className="fs-confirm-row">
                        <button className="btn fs-danger" type="button" onClick={onDelete} disabled={busy}
                                aria-busy={busy}>
                            {busy ? "Deleting..." : "Yes, delete it"}
                        </button>
                        <button className="btn btn-ghost" type="button" onClick={() => setConfirming(false)}
                                disabled={busy}>
                            Cancel
                        </button>
                    </div>
                </div>
            ) : (
                <div className="acct-actions">
                    <button className="btn fs-danger" type="button" onClick={() => setConfirming(true)}>
                        Delete instance
                    </button>
                </div>
            )}
            <FormAlert message={error}/>
        </section>
    );
}
