import {useEffect, useState} from "react";
import type {FormEvent} from "react";
import Field from "../../components/auth/Field.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {fetchMe, setUsername} from "../../api/account.ts";
import {messageFor} from "../../api/errors.ts";

const RULES = "3 to 16 characters: letters, digits and underscores.";

// The username shown on the account page. Friends find an account by this name.
export default function UsernameCard() {
    const [saved, setSaved] = useState<string | null | undefined>(undefined);
    const [draft, setDraft] = useState("");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [notice, setNotice] = useState<string | null>(null);
    const [loadFailure, setLoadFailure] = useState<string | null>(null);

    useEffect(() => {
        let active = true;
        fetchMe().then((user) => {
            if (active) setSaved(user.username);
        }, (error: unknown) => {
            if (active) setLoadFailure(messageFor(error));
        });
        return () => {
            active = false;
        };
    }, []);

    async function onSave(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const value = draft.trim();
        if (busy || value.length === 0) return;
        setBusy(true);
        setError(null);
        setNotice(null);
        try {
            const name = await setUsername(value);
            setSaved(name);
            setDraft("");
            setNotice("Saved.");
        } catch (err) {
            setError(messageFor(err));
        } finally {
            setBusy(false);
        }
    }

    return (
        <section className="acct-section" aria-labelledby="acct-username">
            <h2 className="acct-subtitle" id="acct-username">Username</h2>
            {loadFailure && <FormAlert message={loadFailure}/>}
            {!loadFailure && saved === undefined && <p className="acct-status" role="status">Loading...</p>}
            {saved !== undefined && !loadFailure && (
                <>
                    <dl className="acct-details">
                        <dt>Current</dt>
                        <dd>{saved ?? "Not set"}</dd>
                    </dl>
                    <form className="acct-form" onSubmit={onSave} aria-busy={busy}>
                        <Field
                            label={saved ? "New username" : "Username"}
                            name="username"
                            autoComplete="username"
                            spellCheck={false}
                            maxLength={16}
                            hint={RULES}
                            value={draft}
                            onChange={(e) => setDraft(e.target.value)}
                        />
                        <FormAlert message={error}/>
                        {notice && <p className="acct-status" role="status">{notice}</p>}
                        <button className="btn btn-primary acct-submit" type="submit"
                                disabled={busy || draft.trim().length === 0}>
                            {busy && <span className="acct-spinner" aria-hidden="true"/>}
                            {busy ? "Saving..." : "Save username"}
                        </button>
                    </form>
                </>
            )}
        </section>
    );
}
