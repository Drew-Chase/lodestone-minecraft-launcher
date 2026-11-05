import {useEffect, useState} from "react";
import type {ReactNode} from "react";
import {Link, useSearchParams} from "react-router";
import {fetchSubscription, openBillingPortal} from "../../api/billing.ts";
import type {Subscription} from "../../api/billing.ts";
import {isSessionEnded} from "../../api/client.ts";
import {ApiError, messageFor} from "../../api/errors.ts";
import FormAlert from "../../components/auth/FormAlert.tsx";
import SessionEndedNotice from "../../components/auth/SessionEndedNotice.tsx";
import {formatDate, formatDevices, formatSharing, formatStorage} from "./format.ts";

const PLAN_NAME_FALLBACK = "Free";

// After checkout the webhook may not have arrived yet, so the plan is polled this often, this many times at most.
const CONFIRM_INTERVAL_MS = 2000;
const CONFIRM_ATTEMPTS = 8;

// The line under the plan name. Trials, renewals and failed payments each read differently.
function statusLine(subscription: Subscription): ReactNode {
    const {status, trial_ends_at, current_period_end, cancel_at_period_end} = subscription;
    switch (status) {
        case "trialing":
            return trial_ends_at ? `Trial ends ${formatDate(trial_ends_at)}` : "Trial in progress";
        case "active":
            if (!current_period_end) return "Active";
            return cancel_at_period_end
                ? `Ends ${formatDate(current_period_end)}`
                : `Renews ${formatDate(current_period_end)}`;
        case "past_due":
            return "Payment failed. Update your payment method in the billing portal.";
        default:
            return `This plan is ${status.replace(/_/g, " ")}. Open the billing portal for details.`;
    }
}

function hasNoPlan(subscription: Subscription): boolean {
    return subscription.plan === "free" || subscription.status === "canceled" || subscription.status === "none";
}

// A failed call. An ended session gets the sign-in link instead of the plain copy.
interface Failure {
    message: string;
    ended: boolean;
}

function failureOf(error: unknown): Failure {
    return {message: messageFor(error), ended: isSessionEnded(error)};
}

function FailureAlert({failure}: { failure: Failure | null }) {
    if (!failure) return null;
    return failure.ended ? <SessionEndedNotice/> : <FormAlert message={failure.message}/>;
}

export default function BillingPage() {
    const [params, setParams] = useSearchParams();
    // The success notice is read once, then the query value is removed so a reload does not show it again.
    const [justStarted] = useState(() => params.get("checkout") === "success");

    const [subscription, setSubscription] = useState<Subscription | null>(null);
    const [loadFailure, setLoadFailure] = useState<Failure | null>(null);
    const [attempt, setAttempt] = useState(0);
    const [polls, setPolls] = useState(0);

    const [portalBusy, setPortalBusy] = useState(false);
    const [portalFailure, setPortalFailure] = useState<Failure | null>(null);
    const [noBillingAccount, setNoBillingAccount] = useState(false);

    // Waiting for the webhook: just checked out, still no plan, and attempts remain.
    const confirming = justStarted && subscription !== null && hasNoPlan(subscription) && polls < CONFIRM_ATTEMPTS;

    useEffect(() => {
        if (!params.has("checkout")) return;
        setParams((previous) => {
            previous.delete("checkout");
            return previous;
        }, {replace: true});
    }, [params, setParams]);

    useEffect(() => {
        let active = true;
        setSubscription(null);
        setLoadFailure(null);
        fetchSubscription().then((data) => {
            if (active) setSubscription(data);
        }, (error: unknown) => {
            if (active) setLoadFailure(failureOf(error));
        });
        return () => {
            active = false;
        };
    }, [attempt]);

    // A failed poll counts as an attempt too, and the next one is scheduled the same way.
    useEffect(() => {
        if (!confirming) return;
        let active = true;
        const timer = setTimeout(() => {
            fetchSubscription().then((data) => {
                if (!active) return;
                setSubscription(data);
                setPolls((n) => n + 1);
            }, () => {
                if (active) setPolls((n) => n + 1);
            });
        }, CONFIRM_INTERVAL_MS);
        return () => {
            active = false;
            clearTimeout(timer);
        };
    }, [confirming, polls]);

    async function onManage() {
        if (portalBusy) return;
        setPortalBusy(true);
        setPortalFailure(null);
        setNoBillingAccount(false);
        try {
            const url = await openBillingPortal();
            // The portal is an external page, so the button stays disabled while it loads.
            window.location.assign(url);
        } catch (error) {
            if (error instanceof ApiError && error.code === "no_billing_account") {
                setNoBillingAccount(true);
            } else if (error instanceof ApiError && error.code === "billing_unavailable") {
                setPortalFailure({message: "Subscriptions are not available yet.", ended: false});
            } else {
                setPortalFailure(failureOf(error));
            }
            setPortalBusy(false);
        }
    }

    if (loadFailure) {
        return (
            <div className="bill-page">
                <h1 className="acct-title">Billing</h1>
                <FailureAlert failure={loadFailure}/>
                <div className="acct-actions">
                    <button className="btn btn-primary acct-submit" type="button" onClick={() => setAttempt((n) => n + 1)}>
                        Try again
                    </button>
                </div>
            </div>
        );
    }

    if (!subscription) {
        return (
            <div className="bill-page">
                <h1 className="acct-title">Billing</h1>
                <p className="acct-status" role="status">Loading your plan...</p>
            </div>
        );
    }

    const noPlan = hasNoPlan(subscription);
    const planName = subscription.name || PLAN_NAME_FALLBACK;

    return (
        <div className="bill-page">
            <h1 className="acct-title">Billing</h1>

            {justStarted && <p className="acct-status" role="status">Your trial has started.</p>}

            <section className="bill-card" aria-labelledby="bill-plan">
                <p className="bill-eyebrow">Current plan</p>
                <h2 className="bill-plan-title" id="bill-plan">{planName}</h2>

                {confirming ? (
                    <p className="bill-status" role="status">Confirming your trial...</p>
                ) : noPlan ? (
                    <p className="bill-status">No active plan</p>
                ) : (
                    <p className="bill-status">
                        {statusLine(subscription)}
                        {subscription.interval && <span className="bill-interval">Billed {subscription.interval}</span>}
                    </p>
                )}

                <ul className="bill-allowances">
                    <li>{formatStorage(subscription.storage_bytes)}</li>
                    <li>{formatDevices(subscription.devices)}</li>
                    <li>{formatSharing(subscription.shares)}</li>
                </ul>

                <div className="bill-actions">
                    {noPlan && !confirming && (
                        <Link className="btn btn-primary" to="/subscribe">Choose a plan</Link>
                    )}
                    {noBillingAccount ? (
                        <>
                            <p className="acct-body">No billing account yet.</p>
                            <Link className="btn btn-primary" to="/subscribe">Choose a plan</Link>
                        </>
                    ) : (
                        <button className="btn btn-ghost" type="button" onClick={onManage} disabled={portalBusy}
                                aria-busy={portalBusy}>
                            {portalBusy && <span className="acct-spinner" aria-hidden="true"/>}
                            {portalBusy ? "Opening..." : "Manage subscription"}
                        </button>
                    )}
                </div>
                <FailureAlert failure={portalFailure}/>
            </section>
        </div>
    );
}
