import {useState} from "react";
import type {FormEvent} from "react";
import {Link, useNavigate, useSearchParams} from "react-router";
import AuthLayout from "../../components/auth/AuthLayout.tsx";
import Field from "../../components/auth/Field.tsx";
import SsoButtons from "../../components/auth/SsoButtons.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {messageFor} from "../../api/errors.ts";
import {safeNextPath} from "../../auth/nextPath.ts";
import {useAuth} from "../../auth/useAuth.ts";

// Messages for the error codes a single sign-on callback sends back on the query string. Any other value shows nothing.
// A Map, not an object: an object would return inherited properties such as __proto__ for a crafted query value.
const SSO_ERRORS: ReadonlyMap<string, string> = new Map([
    ["oauth_denied", "Sign-in was cancelled."],
    ["oauth_state", "This sign-in expired, or it was started in another browser. Try again."],
    ["oauth_token", "The provider could not finish sign-in. Try again."],
    ["oauth_identity", "The provider could not verify this account. Try another sign-in method."],
    ["oauth_account_unverified", "This email already has an account that is not verified. Check your inbox for the link that finishes creating it."],
]);

export default function LoginPage() {
    const {signIn} = useAuth();
    const navigate = useNavigate();
    const [params] = useSearchParams();
    const next = safeNextPath(params.get("next"));
    const ssoError = SSO_ERRORS.get(params.get("error") ?? "") ?? null;

    const [email, setEmail] = useState("");
    const [password, setPassword] = useState("");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);

    async function onSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (busy) return;
        setBusy(true);
        setError(null);
        try {
            await signIn(email.trim(), password);
            navigate(next, {replace: true});
        } catch (err) {
            setError(messageFor(err));
            setBusy(false);
        }
    }

    return (
        <AuthLayout>
            <h1 className="acct-title">Sign in</h1>
            {ssoError && <p className="acct-alert" role="alert">{ssoError}</p>}
            <SsoButtons next={next}/>
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
                    label="Password"
                    type="password"
                    name="password"
                    autoComplete="current-password"
                    required
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                />
                <FormAlert message={error}/>
                <button className="btn btn-primary acct-submit" type="submit" disabled={busy}>
                    {busy && <span className="acct-spinner" aria-hidden="true"/>}
                    {busy ? "Signing in..." : "Sign in"}
                </button>
            </form>
            <div className="acct-footer-links">
                <Link to="/forgot-password">Forgot password?</Link>
                <Link to="/register">Create an account</Link>
            </div>
        </AuthLayout>
    );
}
