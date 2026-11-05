import {useEffect, useState} from "react";
import {Navigate, useNavigate} from "react-router";
import PageShell from "../../components/auth/PageShell.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {ApiError, messageFor} from "../../api/errors.ts";
import {apiRequest} from "../../api/client.ts";
import type {User} from "../../api/types.ts";
import {useAuth} from "../../auth/useAuth.ts";
import {useCooldown} from "../../hooks/useCooldown.ts";
import ChangePasswordForm from "./ChangePasswordForm.tsx";
import SessionsCard from "../../components/auth/SessionsCard.tsx";
import UsernameCard from "../social/UsernameCard.tsx";
import MinecraftCard from "../social/MinecraftCard.tsx";

const RESEND_COOLDOWN_SECONDS = 60;

// Codes that mean this browser no longer holds a usable session.
const SESSION_CODES = new Set(["unauthorized", "refresh_invalid", "refresh_reused", "refresh_expired"]);

function isSessionError(error: unknown): boolean {
    return error instanceof ApiError && SESSION_CODES.has(error.code);
}

type Phase = "checking" | "ready" | "signed_out" | "unavailable";

// The signed-in account. Signed-out visitors are sent to sign-in and returned here afterwards.
export default function AccountPage() {
    const {ensureSession, signOut} = useAuth();
    const navigate = useNavigate();
    const [phase, setPhase] = useState<Phase>("checking");
    const [user, setUser] = useState<User | null>(null);
    const [signingOut, setSigningOut] = useState(false);
    const [failure, setFailure] = useState<string | null>(null);
    const [attempt, setAttempt] = useState(0);

    useEffect(() => {
        // StrictMode runs effects twice in development. ensureSession shares one refresh between the two runs, and the
        // flag stops the first run from writing state after the second has started.
        let active = true;
        setPhase("checking");
        setFailure(null);
        (async () => {
            let sessionUser: User | null;
            try {
                sessionUser = await ensureSession();
            } catch (error) {
                if (!active) return;
                setFailure(messageFor(error));
                setPhase("unavailable");
                return;
            }
            if (!active) return;
            if (!sessionUser) {
                setPhase("signed_out");
                return;
            }

            // The session's user can be stale, for example after the address was verified in another tab.
            let current = sessionUser;
            try {
                current = (await apiRequest<{ user: User }>("/api/me", {auth: true})).user;
            } catch (error) {
                if (!active) return;
                if (isSessionError(error)) {
                    setPhase("signed_out");
                    return;
                }
                // /api/me is unavailable. The page still loads from the session's user.
            }
            if (!active) return;
            setUser(current);
            setPhase("ready");
        })();
        return () => {
            active = false;
        };
    }, [ensureSession, attempt]);

    if (phase === "checking") {
        return (
            <PageShell>
                <p className="acct-status" role="status">Loading your account...</p>
            </PageShell>
        );
    }

    if (phase === "unavailable") {
        return (
            <PageShell>
                <h1 className="acct-title">Your account</h1>
                <FormAlert message={failure}/>
                <div className="acct-actions">
                    <button className="btn btn-primary acct-submit" type="button" onClick={() => setAttempt((n) => n + 1)}>
                        Try again
                    </button>
                </div>
            </PageShell>
        );
    }

    if (phase === "signed_out" && !signingOut) {
        return <Navigate to="/login?next=/account" replace/>;
    }

    if (!user) {
        return (
            <PageShell>
                <p className="acct-status" role="status">Loading your account...</p>
            </PageShell>
        );
    }

    async function onSignOut() {
        setSigningOut(true);
        await signOut();
        navigate("/login", {replace: true});
    }

    return (
        <PageShell>
            <div className="acct-account">
                <h1 className="acct-title">Your account</h1>

                <section className="acct-section" aria-labelledby="acct-details">
                    <h2 className="acct-subtitle" id="acct-details">Details</h2>
                    <dl className="acct-details">
                        <dt>Email</dt>
                        <dd>{user.email}</dd>
                        {user.display_name && (
                            <>
                                <dt>Display name</dt>
                                <dd>{user.display_name}</dd>
                            </>
                        )}
                        <dt>Email status</dt>
                        <dd>
                            {user.email_verified
                                ? <span className="chip green">Verified</span>
                                : <span className="chip amber">Not verified</span>}
                        </dd>
                    </dl>
                    {!user.email_verified && <VerificationPrompt email={user.email}/>}
                </section>

                <UsernameCard/>

                <MinecraftCard/>

                <ChangePasswordForm/>

                <SessionsCard/>

                <section className="acct-section" aria-labelledby="acct-session">
                    <h2 className="acct-subtitle" id="acct-session">Session</h2>
                    <button className="btn btn-ghost acct-submit" type="button" onClick={onSignOut} disabled={signingOut}>
                        {signingOut && <span className="acct-spinner" aria-hidden="true"/>}
                        {signingOut ? "Signing out..." : "Sign out"}
                    </button>
                </section>
            </div>
        </PageShell>
    );
}

// Offered only while the address is unverified. The reply is the same whether or not the link was sent.
function VerificationPrompt({email}: { email: string }) {
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
            await resendVerification(email);
            setSent(true);
        } catch (err) {
            setError(messageFor(err));
        } finally {
            setBusy(false);
        }
    }

    const waiting = cooldown.remaining > 0;

    return (
        <div className="acct-actions">
            <p className="acct-body">Verify your email before you use billing, friends or sharing.</p>
            <button className="btn btn-primary acct-submit" type="button" onClick={resend}
                    disabled={busy || waiting} aria-busy={busy}>
                {busy && <span className="acct-spinner" aria-hidden="true"/>}
                {waiting ? `Send again in ${cooldown.remaining}s` : busy ? "Sending..." : "Send verification email"}
            </button>
            {sent && <p className="acct-status" role="status">Check your email for a new link.</p>}
            <FormAlert message={error}/>
        </div>
    );
}
