import {useEffect, useState} from "react";
import type {FormEvent} from "react";
import Field from "../../components/auth/Field.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {fetchMe} from "../../api/account.ts";
import type {AccountUser} from "../../api/account.ts";
import {messageFor} from "../../api/errors.ts";
import {
    acceptFriendRequest,
    createInvite,
    getFriendCode,
    listFriends,
    removeFriendship,
    requestFriendByCode,
    requestFriendByUsername,
    resetFriendCode,
    searchUser,
} from "../../api/friends.ts";
import type {FriendEntry, FriendLists, Invite, SendRequestResult} from "../../api/friends.ts";
import {copyText, formatFriendCode, formatWhen} from "../social/format.ts";
import GateNotice, {gateOfError} from "../../components/social/GateNotice.tsx";
import type {Gate} from "../../components/social/GateNotice.tsx";

// Why adding friends is closed for this account. The user object decides first, and the API's own answers
// (email_not_verified, username_required) lead to the same notice.
function gateFor(user: AccountUser): Gate | null {
    if (!user.email_verified) return "email";
    if (user.username === null) return "username";
    return null;
}

function outcomeText(result: SendRequestResult, fallback: string): string {
    if ("friend" in result) return `You are now friends with ${result.friend.username ?? fallback}.`;
    return `Friend request sent to ${result.request.username ?? fallback}.`;
}

// Shared by the friend sections. A failure reloads the user and the lists, so a gate that the server reports
// appears at once. The function never throws.
interface Refresh {
    (): Promise<void>;
}

interface PageData {
    me: AccountUser;
    code: string;
    lists: FriendLists;
}

export default function FriendsPage() {
    const [data, setData] = useState<PageData | null>(null);
    const [loadFailure, setLoadFailure] = useState<string | null>(null);
    const [attempt, setAttempt] = useState(0);

    useEffect(() => {
        let active = true;
        setLoadFailure(null);
        Promise.all([fetchMe(), getFriendCode(), listFriends()]).then(([me, code, lists]) => {
            if (active) setData({me, code, lists});
        }, (error: unknown) => {
            if (active) setLoadFailure(messageFor(error));
        });
        return () => {
            active = false;
        };
    }, [attempt]);

    if (loadFailure) {
        return (
            <div className="soc-page">
                <h1 className="acct-title">Friends</h1>
                <FormAlert message={loadFailure}/>
                <div className="acct-actions">
                    <button className="btn btn-primary acct-submit" type="button" onClick={() => setAttempt((n) => n + 1)}>
                        Try again
                    </button>
                </div>
            </div>
        );
    }

    if (!data) {
        return (
            <div className="soc-page">
                <h1 className="acct-title">Friends</h1>
                <p className="acct-status" role="status">Loading your friends...</p>
            </div>
        );
    }

    return <FriendsContent initial={data}/>;
}

function FriendsContent({initial}: { initial: PageData }) {
    const [me, setMe] = useState<AccountUser>(initial.me);
    const [code, setCode] = useState(initial.code);
    const [lists, setLists] = useState<FriendLists>(initial.lists);
    const gate = gateFor(me);

    const refresh: Refresh = async () => {
        try {
            const [freshMe, freshLists] = await Promise.all([fetchMe(), listFriends()]);
            setMe(freshMe);
            setLists(freshLists);
        } catch {
            // The shown data stays as it is until the next successful load.
        }
    };

    return (
        <div className="soc-page">
            <h1 className="acct-title">Friends</h1>

            <FriendCodeSection code={code} onNewCode={setCode}/>

            <AddFriendSection gate={gate} refresh={refresh}/>

            <InviteSection gate={gate} refresh={refresh}/>

            <RequestsSection lists={lists} setLists={setLists} refresh={refresh}/>

            <FriendsListSection lists={lists} setLists={setLists} refresh={refresh}/>
        </div>
    );
}

function FriendCodeSection({code, onNewCode}: { code: string; onNewCode: (code: string) => void }) {
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [copyState, setCopyState] = useState<"copied" | "failed" | null>(null);

    async function onCopy() {
        const ok = await copyText(formatFriendCode(code));
        setCopyState(ok ? "copied" : "failed");
    }

    async function onReset() {
        if (busy) return;
        setBusy(true);
        setError(null);
        setCopyState(null);
        try {
            onNewCode(await resetFriendCode());
        } catch (err) {
            setError(messageFor(err));
        } finally {
            setBusy(false);
        }
    }

    const display = formatFriendCode(code);

    return (
        <section className="acct-section" aria-labelledby="soc-code-title">
            <h2 className="acct-subtitle" id="soc-code-title">Your friend code</h2>
            <p className="acct-body">Share this code so a friend can send you a request.</p>
            <div className="soc-code-row">
                <p className="soc-code" aria-label={`Friend code ${display}`}>{display}</p>
                <button className="btn btn-ghost" type="button" onClick={onCopy}>Copy</button>
                <button className="btn btn-ghost" type="button" onClick={onReset} disabled={busy}>
                    {busy ? "Resetting..." : "Reset code"}
                </button>
            </div>
            {copyState === "copied" && <p className="acct-status" role="status">Copied.</p>}
            {copyState === "failed" && (
                <p className="acct-alert" role="alert">Could not copy. Select the code and copy it.</p>
            )}
            <FormAlert message={error}/>
        </section>
    );
}

function AddFriendSection({gate, refresh}: { gate: Gate | null; refresh: Refresh }) {
    const [username, setUsername] = useState("");
    const [found, setFound] = useState<string | null>(null);
    const [friendCode, setFriendCode] = useState("");
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [errorGate, setErrorGate] = useState<Gate | null>(null);
    const [notice, setNotice] = useState<string | null>(null);

    function fail(err: unknown) {
        setError(messageFor(err));
        const blocked = gateOfError(err);
        setErrorGate(blocked);
        if (blocked) void refresh();
    }

    async function onFind(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        const name = username.trim();
        if (busy || name.length === 0) return;
        setBusy(true);
        setError(null);
        setErrorGate(null);
        setNotice(null);
        setFound(null);
        try {
            setFound((await searchUser(name)) ?? name);
        } catch (err) {
            fail(err);
        } finally {
            setBusy(false);
        }
    }

    async function onSendToFound() {
        if (busy || found === null) return;
        await send(() => requestFriendByUsername(found), found);
        setFound(null);
        setUsername("");
    }

    async function onSendByCode(event: FormEvent<HTMLFormElement>) {
        event.preventDefault();
        // Codes are shown as XXXX-XXXX, but the API matches the bare eight characters.
        const value = friendCode.replace(/[\s-]/g, "").toUpperCase();
        if (busy || value.length === 0) return;
        await send(() => requestFriendByCode(value), "them");
        setFriendCode("");
    }

    async function send(action: () => Promise<SendRequestResult>, name: string) {
        setBusy(true);
        setError(null);
        setErrorGate(null);
        setNotice(null);
        try {
            setNotice(outcomeText(await action(), name));
            await refresh();
        } catch (err) {
            fail(err);
        } finally {
            setBusy(false);
        }
    }

    return (
        <section className="acct-section" aria-labelledby="soc-add-title">
            <h2 className="acct-subtitle" id="soc-add-title">Add a friend</h2>
            {gate ? (
                <GateNotice gate={gate}/>
            ) : (
                <>
                    <form className="acct-form soc-form" onSubmit={onFind} aria-busy={busy}>
                        <div className="soc-inline">
                            <Field
                                label="Username"
                                name="friend_username"
                                autoComplete="off"
                                value={username}
                                onChange={(e) => setUsername(e.target.value)}
                            />
                            <button className="btn btn-ghost soc-inline-button" type="submit" disabled={busy}>
                                Find
                            </button>
                        </div>
                    </form>
                    {found !== null && (
                        <div className="soc-result">
                            <span className="soc-name">{found}</span>
                            <button className="btn btn-primary" type="button" onClick={onSendToFound} disabled={busy}>
                                Send request
                            </button>
                        </div>
                    )}
                    <form className="acct-form soc-form" onSubmit={onSendByCode} aria-busy={busy}>
                        <div className="soc-inline">
                            <Field
                                label="Friend code"
                                name="friend_code"
                                autoComplete="off"
                                spellCheck={false}
                                value={friendCode}
                                onChange={(e) => setFriendCode(e.target.value)}
                            />
                            <button className="btn btn-primary soc-inline-button" type="submit" disabled={busy}>
                                Send request
                            </button>
                        </div>
                    </form>
                    <FormAlert message={error}/>
                    {errorGate && <GateNotice gate={errorGate}/>}
                    {notice && <p className="acct-status" role="status">{notice}</p>}
                </>
            )}
        </section>
    );
}

function InviteSection({gate, refresh}: { gate: Gate | null; refresh: Refresh }) {
    const [invite, setInvite] = useState<Invite | null>(null);
    const [busy, setBusy] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [errorGate, setErrorGate] = useState<Gate | null>(null);
    const [copyState, setCopyState] = useState<"copied" | "failed" | null>(null);

    async function onCreate() {
        if (busy) return;
        setBusy(true);
        setError(null);
        setErrorGate(null);
        setCopyState(null);
        try {
            setInvite(await createInvite());
        } catch (err) {
            setError(messageFor(err));
            const blocked = gateOfError(err);
            setErrorGate(blocked);
            if (blocked) void refresh();
        } finally {
            setBusy(false);
        }
    }

    async function onCopy() {
        if (!invite) return;
        const ok = await copyText(invite.url);
        setCopyState(ok ? "copied" : "failed");
    }

    return (
        <section className="acct-section" aria-labelledby="soc-invite-title">
            <h2 className="acct-subtitle" id="soc-invite-title">Invite link</h2>
            <p className="acct-body">A link that adds you as friends when someone opens it while signed in.</p>
            {gate ? (
                <GateNotice gate={gate}/>
            ) : (
                <>
                    <div className="soc-actions-row">
                        <button className="btn btn-primary" type="button" onClick={onCreate} disabled={busy}>
                            {busy ? "Creating..." : "Create invite link"}
                        </button>
                    </div>
                    {invite && (
                        <div className="soc-invite">
                            <div className="soc-inline">
                                <input
                                    className="acct-input soc-url"
                                    type="text"
                                    readOnly
                                    value={invite.url}
                                    aria-label="Invite link"
                                    onFocus={(e) => e.currentTarget.select()}
                                />
                                <button className="btn btn-ghost soc-inline-button" type="button" onClick={onCopy}>
                                    Copy
                                </button>
                            </div>
                            <p className="acct-hint">Expires {formatWhen(invite.expires_at)}</p>
                            {copyState === "copied" && <p className="acct-status" role="status">Copied.</p>}
                            {copyState === "failed" && (
                                <p className="acct-alert" role="alert">Could not copy. Select the link and copy it.</p>
                            )}
                        </div>
                    )}
                    <FormAlert message={error}/>
                    {errorGate && <GateNotice gate={errorGate}/>}
                </>
            )}
        </section>
    );
}

interface ListProps {
    lists: FriendLists;
    setLists: (lists: FriendLists) => void;
    refresh: Refresh;
}

// Accept, decline, cancel and remove share one busy flag, so only one list change runs at a time.
function useListAction(setLists: (lists: FriendLists) => void, refresh: Refresh) {
    const [busyKey, setBusyKey] = useState<string | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [errorGate, setErrorGate] = useState<Gate | null>(null);

    async function run(key: string, action: () => Promise<void>) {
        if (busyKey !== null) return;
        setBusyKey(key);
        setError(null);
        setErrorGate(null);
        try {
            await action();
            const fresh = await listFriends();
            setLists(fresh);
        } catch (err) {
            setError(messageFor(err, {not_found: "That request is no longer here. Refresh the page."}));
            const blocked = gateOfError(err);
            setErrorGate(blocked);
            await refresh();
        } finally {
            setBusyKey(null);
        }
    }

    return {busyKey, error, errorGate, run};
}

function nameOf(entry: FriendEntry): string {
    return entry.username ?? "No username";
}

function RequestsSection({lists, setLists, refresh}: ListProps) {
    const {busyKey, error, errorGate, run} = useListAction(setLists, refresh);
    const hasAny = lists.incoming.length > 0 || lists.outgoing.length > 0;

    return (
        <section className="acct-section" aria-labelledby="soc-requests-title">
            <h2 className="acct-subtitle" id="soc-requests-title">Requests</h2>
            {!hasAny && <p className="acct-body">No pending requests.</p>}

            {lists.incoming.length > 0 && (
                <>
                    <h3 className="soc-subhead">Incoming</h3>
                    <ul className="soc-list">
                        {lists.incoming.map((entry) => (
                            <li className="soc-row" key={entry.id}>
                                <span className="soc-name">{nameOf(entry)}</span>
                                <div className="soc-row-actions">
                                    <button className="btn btn-primary soc-action" type="button"
                                            disabled={busyKey !== null}
                                            onClick={() => run(`accept:${entry.id}`, () => acceptFriendRequest(entry.id))}>
                                        Accept
                                    </button>
                                    <button className="btn btn-ghost soc-action" type="button"
                                            disabled={busyKey !== null}
                                            onClick={() => run(`decline:${entry.id}`, () => removeFriendship(entry.id))}>
                                        Decline
                                    </button>
                                </div>
                            </li>
                        ))}
                    </ul>
                </>
            )}

            {lists.outgoing.length > 0 && (
                <>
                    <h3 className="soc-subhead">Sent</h3>
                    <ul className="soc-list">
                        {lists.outgoing.map((entry) => (
                            <li className="soc-row" key={entry.id}>
                                <span className="soc-name">{nameOf(entry)}</span>
                                <div className="soc-row-actions">
                                    <button className="btn btn-ghost soc-action" type="button"
                                            disabled={busyKey !== null}
                                            onClick={() => run(`cancel:${entry.id}`, () => removeFriendship(entry.id))}>
                                        Cancel
                                    </button>
                                </div>
                            </li>
                        ))}
                    </ul>
                </>
            )}

            <FormAlert message={error}/>
            {errorGate && <GateNotice gate={errorGate}/>}
        </section>
    );
}

function FriendsListSection({lists, setLists, refresh}: ListProps) {
    const {busyKey, error, errorGate, run} = useListAction(setLists, refresh);

    return (
        <section className="acct-section" aria-labelledby="soc-friends-title">
            <h2 className="acct-subtitle" id="soc-friends-title">Friends</h2>
            {lists.friends.length === 0 && <p className="acct-body">You have no friends yet.</p>}
            {lists.friends.length > 0 && (
                <ul className="soc-list">
                    {lists.friends.map((entry) => (
                        <li className="soc-row" key={entry.id}>
                            <span className={entry.username ? "soc-name" : "soc-name soc-name-muted"}>{nameOf(entry)}</span>
                            <div className="soc-row-actions">
                                <button className="btn btn-ghost soc-action" type="button"
                                        disabled={busyKey !== null}
                                        onClick={() => run(`remove:${entry.id}`, () => removeFriendship(entry.id))}>
                                    Remove
                                </button>
                            </div>
                        </li>
                    ))}
                </ul>
            )}
            <FormAlert message={error}/>
            {errorGate && <GateNotice gate={errorGate}/>}
        </section>
    );
}
