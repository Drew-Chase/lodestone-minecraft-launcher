import {useState} from "react";
import {Link} from "react-router";
import AuthLayout from "../../components/auth/AuthLayout.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {messageFor} from "../../api/errors.ts";
import {useAuth} from "../../auth/useAuth.ts";
import {useCooldown} from "../../hooks/useCooldown.ts";

const RESEND_COOLDOWN_SECONDS = 60;

// Shown after registration. The same answer comes back whether or not the address already exists, so the copy says
// only that an email was sent.
export default function CheckEmail({address}: { address: string }) {
    const {resendVerification} = useAuth();
    const cooldown = useCooldown(RESEND_COOLDOWN_SECONDS);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [sent, setSent] = useState(false);

    async function resend() {
        if (busy || cooldown.remaining > 0) return;
        cooldown.start();
        setBusy(true);
        setError(null);
        setSent(false);
        try {
            await resendVerification(address);
            setSent(true);
        } catch (err) {
            setError(messageFor(err));
        } finally {
            setBusy(false);
        }
    }

    const waiting = cooldown.remaining > 0;

    return (
        <AuthLayout>
            <h1 className="acct-title">Check your email</h1>
            <p className="acct-body">
                We sent a verification link to <strong className="acct-strong">{address}</strong>. The link lasts 24 hours.
            </p>
            <div className="acct-actions">
                <button className="btn btn-ghost acct-submit" type="button" onClick={resend}
                        disabled={busy || waiting} aria-busy={busy}>
                    {busy && <span className="acct-spinner" aria-hidden="true"/>}
                    {waiting ? `Resend in ${cooldown.remaining}s` : busy ? "Sending..." : "Resend email"}
                </button>
            </div>
            {sent && <p className="acct-status" role="status">If the address is new, a new link is on its way.</p>}
            <FormAlert message={error}/>
            <p className="acct-footer-link">
                <Link to="/login">Back to sign in</Link>
            </p>
        </AuthLayout>
    );
}
