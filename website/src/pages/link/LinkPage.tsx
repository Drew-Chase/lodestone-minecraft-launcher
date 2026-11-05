import {useEffect, useState} from "react";
import type {FormEvent} from "react";
import {Link, Navigate, useSearchParams} from "react-router";
import AuthLayout from "../../components/auth/AuthLayout.tsx";
import Field from "../../components/auth/Field.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {approveDevice, lookupDevice} from "../../api/devices.ts";
import type {DeviceLookup} from "../../api/devices.ts";
import {ApiError, messageFor} from "../../api/errors.ts";
import {useAuth} from "../../auth/useAuth.ts";

// The launcher's sign-in link carries the code in ?code=. The code is kept here across the sign-in redirect, so it
// never has to stay in the address bar.
const PENDING_CODE_KEY = "lodestone.deviceCode";

function readPendingCode(): string {
    try {
        return sessionStorage.getItem(PENDING_CODE_KEY) ?? "";
    } catch {
        return "";
    }
}

type Phase = "checking" | "ready" | "signed_out" | "unavailable";

// Approves a launcher sign-in in two steps: the code is checked first, so the person sees the device before approving.
// Signed-out visitors go to sign-in and come back here with the code kept.
export default function LinkPage() {
    const {ensureSession} = useAuth();
    const [params] = useSearchParams();

    const [phase, setPhase] = useState<Phase>("checking");
    const [loadFailure, setLoadFailure] = useState<string | null>(null);
    const [attempt, setAttempt] = useState(0);

    const [code, setCode] = useState(() => params.get("code") ?? readPendingCode());
    const [device, setDevice] = useState<DeviceLookup | null>(null);
    const [checking, setChecking] = useState(false);
    const [approving, setApproving] = useState(false);
    const [approved, setApproved] = useState(false);
    const [failure, setFailure] = useState<string | null>(null);
    const [failureCode, setFailureCode] = useState<string | null>(null);

    useEffect(() => {
        // The code is moved out of the address bar and kept for the sign-in redirect. Replacing the URL keeps the
        // history entry's own state, which the router reads.
        const fromLink = params.get("code");
        if (!fromLink) return;
        try {
            sessionStorage.setItem(PENDING_CODE_KEY, fromLink);
        } catch {
            // Without storage the code is still in the field, but a sign-in redirect would drop it.
        }
        window.history.replaceState(window.history.state, "", window.location.pathname);
    }, [params]);

    useEffect(() => {
        // StrictMode runs this twice. ensureSession shares one refresh, and the flag stops the first run from writing
        // state after the second has started.
        let active = true;
        setPhase("checking");
        setLoadFailure(null);
        ensureSession().then((user) => {
            if (!active) return;
            setPhase(user ? "ready" : "signed_out");
        }, (error: unknown) => {
            if (!active) return;
            setLoadFailure(messageFor(error));
            setPhase("unavailable");
        });
        return () => {
            active = false;
        };
    }, [ensureSession, attempt]);

    if (phase === "checking") {
        return (
            <AuthLayout>
                <h1 className="acct-title">Approve sign-in</h1>
                <p className="acct-status" role="status">Loading...</p>
            </AuthLayout>
        );
    }

    if (phase === "unavailable") {
        return (
            <AuthLayout>
                <h1 className="acct-title">Approve sign-in</h1>
                <FormAlert message={loadFailure}/>
                <div className="acct-actions">
                    <button className="btn btn-primary acct-submit" type="button" onClick={() => setAttempt((n) => n + 1)}>
                        Try again
                    </button>
                </div>
            </AuthLayout>
        );
    }

    if (phase === "signed_out") {
        return <Navigate to="/login?next=/link" replace/>;
    }

    // Step one: read the device behind the code. Nothing is approved here.
    async function onCheck(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const value = code.trim();
        if (checking || value.length === 0) return;
        setChecking(true);
        setFailure(null);
        setFailureCode(null);
        try {
            setDevice(await lookupDevice(value));
        } catch (error) {
            setDevice(null);
            setFailure(messageFor(error));
            setFailureCode(error instanceof ApiError ? error.code : null);
        } finally {
            setChecking(false);
        }
    }

    // Step two: approve the code that was checked.
    async function onApprove() {
        if (approving || !device) return;
        setApproving(true);
        setFailure(null);
        setFailureCode(null);
        try {
            await approveDevice(code.trim());
            try {
                sessionStorage.removeItem(PENDING_CODE_KEY);
            } catch {
                // Nothing else depends on the stored code once it is approved.
            }
            setApproved(true);
        } catch (error) {
            setFailure(messageFor(error));
            setFailureCode(error instanceof ApiError ? error.code : null);
        } finally {
            setApproving(false);
        }
    }

    // Back to the code field. The code stays in the field so it can be checked again.
    function onCancel() {
        setDevice(null);
        setFailure(null);
        setFailureCode(null);
    }

    return (
        <AuthLayout>
            <h1 className="acct-title">Approve sign-in</h1>
            {approved ? (
                <p className="acct-status" role="status">
                    Approved. Return to the Lodestone launcher to finish signing in.
                </p>
            ) : device ? (
                <div className="acct-form">
                    <div className="soc-device">
                        <div className="soc-device-info">
                            <span className="soc-device-name">{device.name}</span>
                            <span className="acct-hint">{device.os} · Launcher {device.app_version}</span>
                            <span className="acct-hint">Expires at {new Date(device.expires_at).toLocaleString()}</span>
                        </div>
                    </div>
                    <p className="acct-body">
                        Only approve this if you just started signing in to the Lodestone launcher on your own computer.
                        If you did not start this, do not approve it.
                    </p>
                    <FormAlert message={failure}/>
                    {failureCode === "device_limit_reached" && (
                        <p className="acct-body">
                            <Link to="/dashboard/devices">Manage your devices</Link>
                        </p>
                    )}
                    <div className="acct-actions">
                        <button className="btn btn-primary acct-submit" type="button" onClick={onApprove} disabled={approving}>
                            {approving && <span className="acct-spinner" aria-hidden="true"/>}
                            {approving ? "Approving..." : "Approve this device"}
                        </button>
                        <button className="btn btn-ghost acct-submit" type="button" onClick={onCancel} disabled={approving}>
                            Cancel
                        </button>
                    </div>
                </div>
            ) : (
                <form className="acct-form" onSubmit={onCheck} aria-busy={checking}>
                    <Field
                        label="Code"
                        name="user_code"
                        autoComplete="off"
                        autoCapitalize="characters"
                        spellCheck={false}
                        maxLength={16}
                        required
                        autoFocus
                        hint="Enter the code shown in the Lodestone launcher."
                        value={code}
                        onChange={(e) => setCode(e.target.value)}
                    />
                    <FormAlert message={failure}/>
                    {failureCode === "device_limit_reached" && (
                        <p className="acct-body">
                            <Link to="/dashboard/devices">Manage your devices</Link>
                        </p>
                    )}
                    <button className="btn btn-primary acct-submit" type="submit" disabled={checking}>
                        {checking && <span className="acct-spinner" aria-hidden="true"/>}
                        {checking ? "Checking..." : "Check code"}
                    </button>
                </form>
            )}
        </AuthLayout>
    );
}
