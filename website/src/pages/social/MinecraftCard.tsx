import {useEffect, useState} from "react";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {ApiError, messageFor, messageForCode} from "../../api/errors.ts";
import {
    getMinecraftLinkStatus,
    listMinecraftAccounts,
    startMinecraftLink,
    unlinkMinecraftAccount,
} from "../../api/minecraft.ts";
import type {LinkStart, MinecraftAccount} from "../../api/minecraft.ts";
import {formatWhen} from "./format.ts";

// Used when the server does not send an interval, and when a poll fails without a Retry-After.
const DEFAULT_INTERVAL_SECONDS = 5;

// Lists the Minecraft accounts linked to this Lodestone account, and links new ones through the Microsoft device
// page.
export default function MinecraftCard() {
    const [accounts, setAccounts] = useState<MinecraftAccount[] | null>(null);
    const [loadFailure, setLoadFailure] = useState<string | null>(null);
    const [pending, setPending] = useState<LinkStart | null>(null);
    const [starting, setStarting] = useState(false);
    const [notice, setNotice] = useState<string | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [unlinking, setUnlinking] = useState<string | null>(null);
    const [retrying, setRetrying] = useState(false);

    useEffect(() => {
        let active = true;
        listMinecraftAccounts().then((list) => {
            if (active) setAccounts(list);
        }, (err: unknown) => {
            if (active) setLoadFailure(messageFor(err));
        });
        return () => {
            active = false;
        };
    }, []);

    async function reload(): Promise<void> {
        try {
            setAccounts(await listMinecraftAccounts());
        } catch (err) {
            setError(messageFor(err));
        }
    }

    // Polls the link until it ends. The effect is replaced when `pending` changes, and cleanup stops the timer, so a
    // link that is cancelled or leaves the page is not polled.
    useEffect(() => {
        if (!pending) return;
        const linkId = pending.id;
        let active = true;
        let timer: ReturnType<typeof setTimeout> | undefined;
        let delaySeconds = pending.interval > 0 ? pending.interval : DEFAULT_INTERVAL_SECONDS;
        // Failed checks in a row. A transient failure keeps the same interval and only changes the copy.
        let failures = 0;

        function schedule(seconds: number) {
            timer = setTimeout(check, seconds * 1000);
        }

        async function check() {
            try {
                const result = await getMinecraftLinkStatus(linkId);
                if (!active) return;
                failures = 0;
                setRetrying(false);
                if (result.status === "pending") {
                    if (result.interval && result.interval > 0) delaySeconds = result.interval;
                    schedule(delaySeconds);
                    return;
                }
                setPending(null);
                if (result.status === "linked") {
                    setNotice(`Linked ${result.account?.name ?? "your Minecraft account"}.`);
                    setError(null);
                    await reload();
                } else if (result.status === "denied") {
                    setError("The Microsoft sign-in was denied.");
                } else if (result.status === "expired") {
                    setError("This code expired. Start linking again.");
                } else {
                    setError(result.error ? messageForCode(result.error) : "Linking failed. Try again.");
                }
            } catch (err) {
                if (!active) return;
                // A check that came too soon waits for the server's Retry-After, then tries again.
                if (err instanceof ApiError && err.status === 429) {
                    schedule(err.retryAfter ?? delaySeconds);
                    return;
                }
                // A dropped connection or a 502 is retried on the same interval. Other failures end the link.
                if (err instanceof ApiError && (err.code === "network_error" || err.status === 502)) {
                    failures += 1;
                    if (failures >= 2) setRetrying(true);
                    schedule(delaySeconds);
                    return;
                }
                setPending(null);
                setError(messageFor(err));
            }
        }

        schedule(delaySeconds);
        return () => {
            active = false;
            if (timer !== undefined) clearTimeout(timer);
        };
    }, [pending]);

    async function onLink() {
        if (starting || pending) return;
        setStarting(true);
        setRetrying(false);
        setNotice(null);
        setError(null);
        try {
            setPending(await startMinecraftLink());
        } catch (err) {
            setError(messageFor(err));
        } finally {
            setStarting(false);
        }
    }

    async function onUnlink(uuid: string) {
        if (unlinking !== null) return;
        setUnlinking(uuid);
        setNotice(null);
        setError(null);
        try {
            await unlinkMinecraftAccount(uuid);
            await reload();
        } catch (err) {
            setError(messageFor(err));
        } finally {
            setUnlinking(null);
        }
    }

    return (
        <section className="acct-section" aria-labelledby="acct-minecraft">
            <h2 className="acct-subtitle" id="acct-minecraft">Minecraft accounts</h2>

            {loadFailure && <FormAlert message={loadFailure}/>}
            {!loadFailure && accounts === null && <p className="acct-status" role="status">Loading accounts...</p>}

            {accounts !== null && accounts.length === 0 && !pending && (
                <p className="acct-body">No Minecraft accounts are linked.</p>
            )}
            {accounts !== null && accounts.length > 0 && (
                <ul className="acct-session-list">
                    {accounts.map((account) => (
                        <li className="acct-session" key={account.uuid}>
                            <div className="acct-session-info">
                                <span className="acct-session-name">{account.name}</span>
                                <span className="acct-hint">Linked {formatWhen(account.linked_at)}</span>
                            </div>
                            <button
                                className="btn btn-ghost acct-session-action"
                                type="button"
                                aria-label={`Unlink ${account.name}`}
                                disabled={unlinking !== null}
                                onClick={() => onUnlink(account.uuid)}
                            >
                                {unlinking === account.uuid ? "Unlinking..." : "Unlink"}
                            </button>
                        </li>
                    ))}
                </ul>
            )}

            {pending && (
                <div className="soc-link" role="group" aria-label="Link code">
                    <p className="acct-body">Enter this code on the Microsoft page, then wait here.</p>
                    <p className="soc-code">{pending.user_code}</p>
                    <p className="acct-body">
                        <a href={pending.verification_uri} target="_blank" rel="noopener noreferrer">
                            {pending.verification_uri}
                        </a>
                    </p>
                    <p className="acct-status" role="status">{retrying ? "Trying again..." : "Waiting for Microsoft..."}</p>
                    <div className="acct-actions">
                        <button className="btn btn-ghost acct-submit" type="button" onClick={() => setPending(null)}>
                            Cancel
                        </button>
                    </div>
                </div>
            )}

            {!pending && (
                <div className="acct-actions">
                    <button className="btn btn-primary acct-submit" type="button" onClick={onLink}
                            disabled={starting} aria-busy={starting}>
                        {starting && <span className="acct-spinner" aria-hidden="true"/>}
                        {starting ? "Starting..." : "Link a Minecraft account"}
                    </button>
                </div>
            )}

            {notice && <p className="acct-status" role="status">{notice}</p>}
            <FormAlert message={error}/>
        </section>
    );
}
