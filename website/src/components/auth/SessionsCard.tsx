import {useCallback, useEffect, useState} from "react";
import {ApiError} from "../../api/errors.ts";
import {listSessions, revokeOtherSessions, revokeSession} from "../../api/sessions.ts";
import type {Session} from "../../api/sessions.ts";

// Copy for the session actions. The shared error table has no entries for these codes, so they are mapped here.
function sessionMessage(error: unknown): string {
    if (error instanceof ApiError) {
        if (error.status === 404) return "That session is no longer active.";
        if (error.status === 409) return "Use Sign out to end this session.";
    }
    return "Something went wrong. Try again.";
}

function formatDate(value: string): string {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

export default function SessionsCard() {
    const [sessions, setSessions] = useState<Session[] | null>(null);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const load = useCallback(async () => {
        try {
            const list = await listSessions();
            setSessions(list.sessions);
            return true;
        } catch (err) {
            setError(sessionMessage(err));
            return false;
        }
    }, []);

    useEffect(() => {
        let active = true;
        listSessions()
            .then((list) => {
                if (active) setSessions(list.sessions);
            })
            .catch((err: unknown) => {
                if (active) setError(sessionMessage(err));
            });
        return () => {
            active = false;
        };
    }, []);

    async function run(action: () => Promise<unknown>) {
        if (busy) return;
        setBusy(true);
        setError(null);
        try {
            await action();
            await load();
        } catch (err) {
            setError(sessionMessage(err));
            // Reload anyway, so the list shows what the server now holds (for example, a session that already ended).
            await load();
        } finally {
            setBusy(false);
        }
    }

    const others = sessions?.filter((session) => !session.current) ?? [];

    return (
        <section className="acct-section" aria-labelledby="acct-sessions">
            <h2 className="acct-subtitle" id="acct-sessions">Sessions</h2>
            {sessions === null && !error && <p className="acct-status" role="status">Loading sessions...</p>}
            {sessions !== null && sessions.length === 0 && (
                <p className="acct-body">No active sessions.</p>
            )}
            {sessions !== null && sessions.length > 0 && (
                <ul className="acct-session-list">
                    {sessions.map((session) => {
                        const signedIn = formatDate(session.created_at);
                        return (
                            <li className="acct-session" key={session.id}>
                                <div className="acct-session-info">
                                    <span className="acct-session-name">
                                        Web browser
                                        {session.current && <span className="chip green acct-session-badge">This session</span>}
                                    </span>
                                    <span className="acct-hint">Signed in {signedIn}</span>
                                    <span className="acct-hint">Last used {formatDate(session.last_used_at)}</span>
                                </div>
                                {!session.current && (
                                    <button
                                        className="btn btn-ghost acct-session-action"
                                        type="button"
                                        aria-label={`Sign out session signed in ${signedIn}`}
                                        disabled={busy}
                                        onClick={() => run(() => revokeSession(session.id))}
                                    >
                                        Sign out
                                    </button>
                                )}
                            </li>
                        );
                    })}
                </ul>
            )}
            {others.length > 0 && (
                <div className="acct-actions">
                    <button
                        className="btn btn-ghost acct-submit"
                        type="button"
                        disabled={busy}
                        onClick={() => run(revokeOtherSessions)}
                    >
                        Sign out other sessions
                    </button>
                </div>
            )}
            {error && <p className="acct-alert" role="alert">{error}</p>}
        </section>
    );
}
