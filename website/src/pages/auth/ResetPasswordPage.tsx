import {useState} from "react";
import type {FormEvent} from "react";
import {Link, useLocation} from "react-router";
import AuthLayout from "../../components/auth/AuthLayout.tsx";
import Field from "../../components/auth/Field.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {messageFor, messageForCode} from "../../api/errors.ts";
import {useAuth} from "../../auth/useAuth.ts";
import {tokenFromHash} from "../../auth/fragmentToken.ts";

const MIN_PASSWORD_LENGTH = 12;

export default function ResetPasswordPage() {
    const {resetPassword} = useAuth();
    const token = tokenFromHash(useLocation().hash);

    const [password, setPassword] = useState("");
    const [confirm, setConfirm] = useState("");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [done, setDone] = useState(false);

    async function onSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (busy) return;
        if (password.length < MIN_PASSWORD_LENGTH) {
            setError(messageForCode("password_too_short"));
            return;
        }
        if (password !== confirm) {
            setError("The two passwords do not match.");
            return;
        }
        setBusy(true);
        setError(null);
        try {
            await resetPassword(token, password);
            setDone(true);
        } catch (err) {
            setError(messageFor(err));
        } finally {
            setBusy(false);
        }
    }

    return (
        <AuthLayout>
            <h1 className="acct-title">Choose a new password</h1>
            {!token && <FormAlert message={messageForCode("invalid_token")}/>}
            {token && done && (
                <>
                    <p className="acct-status" role="status">Your password has been changed.</p>
                    <div className="acct-actions">
                        <Link className="btn btn-primary acct-submit" to="/login">Sign in</Link>
                    </div>
                </>
            )}
            {token && !done && (
                <form className="acct-form" onSubmit={onSubmit} aria-busy={busy} noValidate>
                    <Field
                        label="New password"
                        type="password"
                        name="password"
                        autoComplete="new-password"
                        autoFocus
                        required
                        minLength={MIN_PASSWORD_LENGTH}
                        hint="At least 12 characters"
                        value={password}
                        onChange={(e) => setPassword(e.target.value)}
                    />
                    <Field
                        label="Confirm new password"
                        type="password"
                        name="confirm"
                        autoComplete="new-password"
                        required
                        value={confirm}
                        onChange={(e) => setConfirm(e.target.value)}
                    />
                    <FormAlert message={error}/>
                    <button className="btn btn-primary acct-submit" type="submit" disabled={busy}>
                        {busy && <span className="acct-spinner" aria-hidden="true"/>}
                        {busy ? "Saving..." : "Set new password"}
                    </button>
                </form>
            )}
            <p className="acct-footer-link">
                <Link to="/login">Back to sign in</Link>
            </p>
        </AuthLayout>
    );
}
