import {useState} from "react";
import type {FormEvent} from "react";
import Field from "../../components/auth/Field.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {messageFor, messageForCode} from "../../api/errors.ts";
import {useAuth} from "../../auth/useAuth.ts";

const MIN_PASSWORD_LENGTH = 12;

// On this form a wrong current password is the only sign-in failure, so it gets its own wording.
const CHANGE_OVERRIDES = {invalid_credentials: "Current password is not right."};

export default function ChangePasswordForm() {
    const {changePassword} = useAuth();
    const [current, setCurrent] = useState("");
    const [next, setNext] = useState("");
    const [confirm, setConfirm] = useState("");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [done, setDone] = useState(false);

    async function onSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (busy) return;
        setDone(false);
        if (next.length < MIN_PASSWORD_LENGTH) {
            setError(messageForCode("password_too_short"));
            return;
        }
        if (next !== confirm) {
            setError("The two new passwords do not match.");
            return;
        }
        setBusy(true);
        setError(null);
        try {
            await changePassword(current, next);
            setCurrent("");
            setNext("");
            setConfirm("");
            setDone(true);
        } catch (err) {
            setError(messageFor(err, CHANGE_OVERRIDES));
        } finally {
            setBusy(false);
        }
    }

    return (
        <section className="acct-section" aria-labelledby="acct-change-password">
            <h2 className="acct-subtitle" id="acct-change-password">Change password</h2>
            <form className="acct-form" onSubmit={onSubmit} aria-busy={busy} noValidate>
                <Field
                    label="Current password"
                    type="password"
                    name="current-password"
                    autoComplete="current-password"
                    required
                    value={current}
                    onChange={(e) => setCurrent(e.target.value)}
                />
                <Field
                    label="New password"
                    type="password"
                    name="new-password"
                    autoComplete="new-password"
                    required
                    minLength={MIN_PASSWORD_LENGTH}
                    hint="At least 12 characters"
                    value={next}
                    onChange={(e) => setNext(e.target.value)}
                />
                <Field
                    label="Confirm new password"
                    type="password"
                    name="confirm-password"
                    autoComplete="new-password"
                    required
                    value={confirm}
                    onChange={(e) => setConfirm(e.target.value)}
                />
                <FormAlert message={error}/>
                {done && <p className="acct-status" role="status">Your password has been changed.</p>}
                <button className="btn btn-ghost acct-submit" type="submit" disabled={busy}>
                    {busy && <span className="acct-spinner" aria-hidden="true"/>}
                    {busy ? "Changing..." : "Change password"}
                </button>
            </form>
        </section>
    );
}
