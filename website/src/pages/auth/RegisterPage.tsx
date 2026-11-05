import {useState} from "react";
import type {FormEvent} from "react";
import {Link} from "react-router";
import AuthLayout from "../../components/auth/AuthLayout.tsx";
import Field from "../../components/auth/Field.tsx";
import SsoButtons from "../../components/auth/SsoButtons.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {messageFor} from "../../api/errors.ts";
import {useAuth} from "../../auth/useAuth.ts";
import CheckEmail from "./CheckEmail.tsx";

export default function RegisterPage() {
    const {register} = useAuth();
    const [email, setEmail] = useState("");
    const [displayName, setDisplayName] = useState("");
    const [password, setPassword] = useState("");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [sentTo, setSentTo] = useState<string | null>(null);

    async function onSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (busy) return;
        const address = email.trim();
        setBusy(true);
        setError(null);
        try {
            await register(address, password, displayName.trim() || undefined);
            setSentTo(address);
        } catch (err) {
            setError(messageFor(err));
        } finally {
            setBusy(false);
        }
    }

    if (sentTo) {
        return <CheckEmail address={sentTo}/>;
    }

    return (
        <AuthLayout>
            <h1 className="acct-title">Create your account</h1>
            <SsoButtons/>
            <form className="acct-form" onSubmit={onSubmit} aria-busy={busy}>
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
                <Field
                    label="Display name (optional)"
                    type="text"
                    name="name"
                    autoComplete="name"
                    maxLength={64}
                    value={displayName}
                    onChange={(e) => setDisplayName(e.target.value)}
                />
                <Field
                    label="Password"
                    type="password"
                    name="password"
                    autoComplete="new-password"
                    required
                    minLength={12}
                    hint="At least 12 characters"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                />
                <FormAlert message={error}/>
                <button className="btn btn-primary acct-submit" type="submit" disabled={busy}>
                    {busy && <span className="acct-spinner" aria-hidden="true"/>}
                    {busy ? "Creating account..." : "Create account"}
                </button>
            </form>
            <p className="acct-legal-note">
                By creating an account you agree to the <Link to="/terms">Terms</Link> and{" "}
                <Link to="/privacy">Privacy Policy</Link>.
            </p>
            <p className="acct-footer-link">
                Already have an account? <Link to="/login">Sign in</Link>
            </p>
        </AuthLayout>
    );
}
