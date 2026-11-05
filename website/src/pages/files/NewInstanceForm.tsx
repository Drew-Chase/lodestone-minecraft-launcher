import {useState} from "react";
import type {FormEvent} from "react";
import Field from "../../components/auth/Field.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {createInstance} from "../../api/instances.ts";
import {messageFor} from "../../api/errors.ts";
import SyncNotice, {syncGateOf} from "./SyncNotice.tsx";
import type {SyncGate} from "./SyncNotice.tsx";

// Creates an instance and then calls onCreated so the list reloads.
export default function NewInstanceForm({onCreated}: { onCreated: () => Promise<void> }) {
    const [name, setName] = useState("");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [gate, setGate] = useState<SyncGate | null>(null);
    const [notice, setNotice] = useState<string | null>(null);

    async function onSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const value = name.trim();
        if (busy || value.length === 0) return;
        setBusy(true);
        setError(null);
        setGate(null);
        setNotice(null);
        try {
            const created = await createInstance(value);
            setName("");
            setNotice(`Created ${created.name}.`);
            await onCreated();
        } catch (err) {
            const blocked = syncGateOf(err);
            setGate(blocked);
            if (!blocked) setError(messageFor(err));
        } finally {
            setBusy(false);
        }
    }

    return (
        <section className="acct-section" aria-labelledby="fs-new-title">
            <h2 className="acct-subtitle" id="fs-new-title">New instance</h2>
            <form className="acct-form soc-form" onSubmit={onSubmit} aria-busy={busy}>
                <div className="soc-inline">
                    <Field
                        label="Name"
                        name="instance_name"
                        autoComplete="off"
                        value={name}
                        onChange={(e) => setName(e.target.value)}
                    />
                    <button className="btn btn-primary soc-inline-button" type="submit"
                            disabled={busy || name.trim().length === 0}>
                        {busy ? "Creating..." : "Create instance"}
                    </button>
                </div>
            </form>
            {gate && <SyncNotice gate={gate}/>}
            <FormAlert message={error}/>
            {notice && <p className="acct-status" role="status">{notice}</p>}
        </section>
    );
}
