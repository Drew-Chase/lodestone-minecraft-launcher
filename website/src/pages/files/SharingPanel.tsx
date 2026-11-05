import {useEffect, useState} from "react";
import type {FormEvent} from "react";
import Field from "../../components/auth/Field.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {listShares, removeShare, shareInstance} from "../../api/instances.ts";
import type {Access, Share} from "../../api/instances.ts";
import {messageFor} from "../../api/errors.ts";
import {ACCESS_LABEL} from "./format.ts";
import SyncNotice, {syncGateOf} from "./SyncNotice.tsx";
import type {SyncGate} from "./SyncNotice.tsx";

// Sharing reaches only accepted friends, so a missing account is reported in those terms.
const SHARE_MESSAGES: Readonly<Record<string, string>> = {
    not_found: "That friend was not found. You can only share with accepted friends.",
};

const REMOVE_MESSAGES: Readonly<Record<string, string>> = {
    not_found: "That person no longer has access to this instance.",
};

// Lists who this instance is shared with, and adds, updates or removes a share. Owners only.
export default function SharingPanel({instanceId}: { instanceId: number }) {
    const [shares, setShares] = useState<Share[] | null>(null);
    const [loadFailure, setLoadFailure] = useState<string | null>(null);
    const [username, setUsername] = useState("");
    const [access, setAccess] = useState<Access>("viewer");
    const [busy, setBusy] = useState(false);
    const [removing, setRemoving] = useState<string | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [gate, setGate] = useState<SyncGate | null>(null);
    const [notice, setNotice] = useState<string | null>(null);

    useEffect(() => {
        let active = true;
        setShares(null);
        setLoadFailure(null);
        listShares(instanceId).then((list) => {
            if (active) setShares(list);
        }, (err: unknown) => {
            if (active) setLoadFailure(messageFor(err));
        });
        return () => {
            active = false;
        };
    }, [instanceId]);

    async function reload(): Promise<void> {
        try {
            setShares(await listShares(instanceId));
        } catch (err) {
            setError(messageFor(err));
        }
    }

    async function onShare(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const name = username.trim();
        if (busy || name.length === 0) return;
        setBusy(true);
        setError(null);
        setGate(null);
        setNotice(null);
        try {
            await shareInstance(instanceId, name, access);
            setUsername("");
            setNotice(`Shared with ${name} as ${ACCESS_LABEL[access].toLowerCase()}.`);
            await reload();
        } catch (err) {
            setGate(syncGateOf(err));
            setError(messageFor(err, SHARE_MESSAGES));
        } finally {
            setBusy(false);
        }
    }

    async function onRemove(name: string) {
        if (removing !== null) return;
        setRemoving(name);
        setError(null);
        setNotice(null);
        try {
            await removeShare(instanceId, name);
            await reload();
        } catch (err) {
            setError(messageFor(err, REMOVE_MESSAGES));
        } finally {
            setRemoving(null);
        }
    }

    return (
        <section className="acct-section" aria-labelledby="fs-sharing-title">
            <h2 className="acct-subtitle" id="fs-sharing-title">Sharing</h2>
            <FormAlert message={loadFailure}/>
            {shares === null && loadFailure === null && <p className="acct-status" role="status">Loading shares...</p>}
            {shares !== null && shares.length === 0 && <p className="acct-body">Not shared with anyone.</p>}
            {shares !== null && shares.length > 0 && (
                <ul className="soc-list">
                    {shares.map((share) => (
                        <li className="soc-row" key={share.username}>
                            <div className="fs-share-info">
                                <span className="soc-name">{share.username}</span>
                                <span className={`fs-badge fs-badge-${share.access}`}>{ACCESS_LABEL[share.access]}</span>
                            </div>
                            <div className="soc-row-actions">
                                <button
                                    className="btn btn-ghost soc-action"
                                    type="button"
                                    aria-label={`Remove ${share.username}`}
                                    disabled={removing !== null}
                                    onClick={() => onRemove(share.username)}
                                >
                                    {removing === share.username ? "Removing..." : "Remove"}
                                </button>
                            </div>
                        </li>
                    ))}
                </ul>
            )}

            <form className="acct-form soc-form" onSubmit={onShare} aria-busy={busy}>
                <div className="soc-inline">
                    <Field
                        label="Username"
                        name="share_username"
                        autoComplete="off"
                        value={username}
                        onChange={(e) => setUsername(e.target.value)}
                    />
                    <div className="acct-field fs-access">
                        <label className="acct-label" htmlFor="fs-share-access">Access</label>
                        <select
                            id="fs-share-access"
                            className="acct-input"
                            value={access}
                            onChange={(e) => setAccess(e.target.value as Access)}
                        >
                            <option value="viewer">Viewer</option>
                            <option value="editor">Editor</option>
                        </select>
                    </div>
                    <button className="btn btn-primary soc-inline-button" type="submit"
                            disabled={busy || username.trim().length === 0}>
                        {busy ? "Saving..." : "Share"}
                    </button>
                </div>
            </form>
            {gate && <SyncNotice gate={gate}/>}
            <FormAlert message={error}/>
            {notice && <p className="acct-status" role="status">{notice}</p>}
        </section>
    );
}
