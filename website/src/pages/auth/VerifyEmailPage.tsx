import {useEffect, useState} from "react";
import type {FormEvent} from "react";
import {Link, useLocation} from "react-router";
import AuthLayout from "../../components/auth/AuthLayout.tsx";
import Field from "../../components/auth/Field.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {messageFor, messageForCode} from "../../api/errors.ts";
import {useAuth} from "../../auth/useAuth.ts";
import {tokenFromHash} from "../../auth/fragmentToken.ts";
import {useCooldown} from "../../hooks/useCooldown.ts";

// A verification token works once. StrictMode runs effects twice in development, so each token gets one request, and
// both runs wait on the same promise. The map lives for the page load.
const inFlight = new Map<string, Promise<void>>();

type VerifyState =
    | { kind: "pending" }
    | { kind: "verified" }
    | { kind: "failed"; message: string };

const RESEND_COOLDOWN_SECONDS = 60;

export default function VerifyEmailPage() {
    const {verifyEmail} = useAuth();
    const token = tokenFromHash(useLocation().hash);
    const [state, setState] = useState<VerifyState>(() =>
        token ? {kind: "pending"} : {kind: "failed", message: messageForCode("invalid_token")},
    );

    useEffect(() => {
        if (!token) return;
        let promise = inFlight.get(token);
        if (!promise) {
            promise = verifyEmail(token);
            inFlight.set(token, promise);
        }
        let active = true;
        promise.then(
            () => {
                if (active) setState({kind: "verified"});
            },
            (error: unknown) => {
                if (active) setState({kind: "failed", message: messageFor(error)});
            },
        );
        return () => {
            active = false;
        };
    }, [token, verifyEmail]);

    return (
        <AuthLayout>
            <h1 className="acct-title">Verify your email</h1>
            {state.kind === "pending" && (
                <p className="acct-status" role="status">Verifying your email...</p>
            )}
            {state.kind === "verified" && (
                <>
                    <p className="acct-status" role="status">Your email is verified. You can sign in now.</p>
                    <div className="acct-actions">
                        <Link className="btn btn-primary acct-submit" to="/login">Sign in</Link>
                    </div>
                </>
            )}
            {state.kind === "failed" && (
                <>
                    <FormAlert message={state.message}/>
                    <ResendForm/>
                </>
            )}
        </AuthLayout>
    );
}

// Asks for a new link. The reply is the same whether or not the address exists.
function ResendForm() {
    const {resendVerification} = useAuth();
    const cooldown = useCooldown(RESEND_COOLDOWN_SECONDS);
    const [email, setEmail] = useState("");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [sent, setSent] = useState(false);

    async function onSubmit(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        if (busy || cooldown.remaining > 0) return;
        cooldown.start();
        setBusy(true);
        setError(null);
        setSent(false);
        try {
            await resendVerification(email.trim());
            setSent(true);
        } catch (err) {
            setError(messageFor(err));
        } finally {
            setBusy(false);
        }
    }

    const waiting = cooldown.remaining > 0;

    return (
        <>
            <p className="acct-body">Enter your email and we will send a new link.</p>
            <form className="acct-form" onSubmit={onSubmit} aria-busy={busy}>
                <Field
                    label="Email"
                    type="email"
                    name="email"
                    autoComplete="email"
                    required
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                />
                <FormAlert message={error}/>
                {sent && <p className="acct-status" role="status">If an account exists for that address, we have sent a new link.</p>}
                <button className="btn btn-primary acct-submit" type="submit" disabled={busy || waiting}>
                    {busy && <span className="acct-spinner" aria-hidden="true"/>}
                    {waiting ? `Send again in ${cooldown.remaining}s` : busy ? "Sending..." : "Send a new link"}
                </button>
            </form>
            <p className="acct-footer-link">
                <Link to="/login">Back to sign in</Link>
            </p>
        </>
    );
}
