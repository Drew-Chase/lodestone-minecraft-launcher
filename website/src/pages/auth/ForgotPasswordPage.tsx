import {useState} from "react";
import type {FormEvent} from "react";
import {Link} from "react-router";
import AuthLayout from "../../components/auth/AuthLayout.tsx";
import Field from "../../components/auth/Field.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {messageFor} from "../../api/errors.ts";
import {useAuth} from "../../auth/useAuth.ts";

export default function ForgotPasswordPage() {
    const {requestPasswordReset} = useAuth();
    const [email, setEmail] = useState("");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [sent, setSent] = useState(false);

    async function onSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (busy) return;
        setBusy(true);
        setError(null);
        try {
            await requestPasswordReset(email.trim());
            setSent(true);
        } catch (err) {
            // A rate limit or a network failure is a real error. An unknown address is not, so it never reaches here.
            setError(messageFor(err));
        } finally {
            setBusy(false);
        }
    }

    return (
        <AuthLayout>
            <h1 className="acct-title">Reset your password</h1>
            {sent ? (
                <p className="acct-status" role="status">
                    If an account exists for that address, we have sent a link.
                </p>
            ) : (
                <form className="acct-form" onSubmit={onSubmit} aria-busy={busy}>
                    <p className="acct-body">Enter the email address on your account and we will send a reset link.</p>
                    <Field
                        label="Email"
                        type="email"
                        name="email"
                        autoComplete="email"
                        required
                        autoFocus
                        value={email}
                        onChange={(e) => setEmail(e.target.value)}
                    />
                    <FormAlert message={error}/>
                    <button className="btn btn-primary acct-submit" type="submit" disabled={busy}>
                        {busy && <span className="acct-spinner" aria-hidden="true"/>}
                        {busy ? "Sending..." : "Send reset link"}
                    </button>
                </form>
            )}
            <p className="acct-footer-link">
                <Link to="/login">Back to sign in</Link>
            </p>
        </AuthLayout>
    );
}
