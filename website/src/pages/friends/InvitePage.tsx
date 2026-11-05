import {useEffect, useState} from "react";
import {Link} from "react-router";
import AuthLayout from "../../components/auth/AuthLayout.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import GateNotice, {gateOfCode} from "../../components/social/GateNotice.tsx";
import {acceptInvite} from "../../api/friends.ts";
import type {InviteAccepted} from "../../api/friends.ts";
import {ApiError, messageFor, messageForCode} from "../../api/errors.ts";
import {tokenFromHash} from "../../auth/fragmentToken.ts";
import {useAuth} from "../../auth/useAuth.ts";

// The token waits in sessionStorage while the visitor signs in, so it survives the redirect without a query string.
const INVITE_TOKEN_KEY = "lodestone.inviteToken";

// Codes after which the invite is spent or can never work, so the stored token is discarded. Every other failure keeps
// it, so the visitor can retry the same invite.
const SPENT_CODES = new Set(["invite_used", "invite_expired", "invalid_invite", "invalid_friend", "already_friends"]);

// A StrictMode double run shares one request, so a single-use token is posted once.
const inFlight = new Map<string, Promise<InviteAccepted>>();

type Phase = "checking" | "signed_out" | "signed_in" | "unavailable";

// Reads the token from the fragment, stores it, and removes the fragment from the address bar. It is called from an
// effect. Safe to call twice: the second call finds the token already stored.
function takeInviteToken(): string {
    const fromHash = tokenFromHash(window.location.hash);
    try {
        if (fromHash) {
            sessionStorage.setItem(INVITE_TOKEN_KEY, fromHash);
            window.history.replaceState(window.history.state, "", window.location.pathname + window.location.search);
        }
        return sessionStorage.getItem(INVITE_TOKEN_KEY) ?? "";
    } catch {
        return fromHash;
    }
}

function clearInviteToken(): void {
    try {
        sessionStorage.removeItem(INVITE_TOKEN_KEY);
    } catch {
        // Storage is unavailable, so there is nothing stored to clear.
    }
}

export default function InvitePage() {
    const {ensureSession} = useAuth();
    const [token, setToken] = useState("");
    const [phase, setPhase] = useState<Phase>("checking");
    const [loadFailure, setLoadFailure] = useState<string | null>(null);
    const [attempt, setAttempt] = useState(0);

    // Runs before the session check below, so the token is in state before the phase can reach signed_in.
    useEffect(() => {
        setToken(takeInviteToken());
    }, []);

    useEffect(() => {
        let active = true;
        setPhase("checking");
        setLoadFailure(null);
        ensureSession().then((user) => {
            if (!active) return;
            setPhase(user ? "signed_in" : "signed_out");
        }, (error: unknown) => {
            if (!active) return;
            setLoadFailure(messageFor(error));
            setPhase("unavailable");
        });
        return () => {
            active = false;
        };
    }, [ensureSession, attempt]);

    return (
        <AuthLayout>
            <h1 className="acct-title">Friend invite</h1>
            {phase === "checking" && <p className="acct-status" role="status">Loading...</p>}
            {phase === "unavailable" && (
                <>
                    <FormAlert message={loadFailure}/>
                    <div className="acct-actions">
                        <button className="btn btn-primary acct-submit" type="button" onClick={() => setAttempt((n) => n + 1)}>
                            Try again
                        </button>
                    </div>
                </>
            )}
            {phase === "signed_out" && (
                <>
                    {token ? (
                        <>
                            <p className="acct-body">Sign in to accept this invite.</p>
                            <div className="acct-actions">
                                <Link className="btn btn-primary acct-submit" to="/login?next=/friends/invite">Sign in</Link>
                            </div>
                        </>
                    ) : (
                        <FormAlert message={messageForCode("invalid_invite")}/>
                    )}
                </>
            )}
            {phase === "signed_in" && <Accept token={token}/>}
        </AuthLayout>
    );
}

type AcceptState =
    | { kind: "pending" }
    | { kind: "done"; name: string | null }
    | { kind: "failed"; message: string; code: string | null; kept: boolean };

function Accept({token}: { token: string }) {
    const [state, setState] = useState<AcceptState>(() => (token
        ? {kind: "pending"}
        : {kind: "failed", message: messageForCode("invalid_invite"), code: null, kept: false}));
    const [attempt, setAttempt] = useState(0);

    useEffect(() => {
        if (!token) return;
        let promise = inFlight.get(token);
        if (!promise) {
            promise = acceptInvite(token);
            inFlight.set(token, promise);
            // A settled token is not kept, so a retry after a fixed account makes a new request.
            promise.finally(() => inFlight.delete(token)).catch(() => undefined);
        }
        let active = true;
        promise.then(
            (result) => {
                clearInviteToken();
                if (active) setState({kind: "done", name: result.friend.username});
            },
            (error: unknown) => {
                const code = error instanceof ApiError ? error.code : null;
                const spent = code !== null && SPENT_CODES.has(code);
                if (spent) clearInviteToken();
                if (active) setState({kind: "failed", message: messageFor(error), code, kept: !spent});
            },
        );
        return () => {
            active = false;
        };
    }, [token, attempt]);

    function onRetry() {
        setState({kind: "pending"});
        setAttempt((n) => n + 1);
    }

    if (state.kind === "pending") {
        return <p className="acct-status" role="status">Accepting invite...</p>;
    }

    if (state.kind === "done") {
        return (
            <>
                <p className="acct-status" role="status">
                    {state.name ? `You are now friends with ${state.name}.` : "You are now friends."}
                </p>
                <div className="acct-actions">
                    <Link className="btn btn-primary acct-submit" to="/dashboard/friends">Open friends</Link>
                </div>
            </>
        );
    }

    const gate = gateOfCode(state.code);

    return (
        <>
            {gate ? <GateNotice gate={gate}/> : <FormAlert message={state.message}/>}
            {state.kept && (
                <div className="acct-actions">
                    <button className="btn btn-primary acct-submit" type="button" onClick={onRetry}>
                        Try again
                    </button>
                </div>
            )}
        </>
    );
}
