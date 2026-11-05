import {useEffect, useState} from "react";
import {Link, Navigate, useLocation, useNavigate, useSearchParams} from "react-router";
import {fetchSubscription, startCheckout} from "../../api/billing.ts";
import type {Interval, PlanId, Subscription} from "../../api/billing.ts";
import {isSessionEnded} from "../../api/client.ts";
import {ApiError, messageFor} from "../../api/errors.ts";
import PageShell from "../../components/auth/PageShell.tsx";
import FormAlert from "../../components/auth/FormAlert.tsx";
import SessionEndedNotice from "../../components/auth/SessionEndedNotice.tsx";
import {useAuth} from "../../auth/useAuth.ts";

interface Plan {
    id: PlanId;
    name: string;
    storage: string;
    devices: string;
    sharing: string;
    monthly: string;
    yearly: string;
}

// Prices are text only. Stripe holds the real amounts.
const PLANS: readonly Plan[] = [
    {
        id: "solo", name: "Solo Sync", storage: "100 GB storage", devices: "5 devices", sharing: "No sharing",
        monthly: "$3", yearly: "$29",
    },
    {
        id: "friends", name: "Friends", storage: "250 GB storage", devices: "5 devices",
        sharing: "Share up to 5 instances", monthly: "$6", yearly: "$58",
    },
    {
        id: "unlimited", name: "Unlimited", storage: "500 GB storage", devices: "Unlimited devices",
        sharing: "Unlimited sharing", monthly: "$12", yearly: "$115",
    },
];

const DEFAULT_PLAN: PlanId = "friends";

function parsePlan(value: string | null): PlanId | null {
    return PLANS.find((plan) => plan.id === value)?.id ?? null;
}

function hasPaidPlan(subscription: Subscription): boolean {
    if (subscription.plan === "free") return false;
    return subscription.status === "trialing" || subscription.status === "active" || subscription.status === "past_due";
}

interface Failure {
    text: string;
    link?: { to: string; label: string };
    // An ended session shows the sign-in link in place of the text.
    signIn?: boolean;
}

function checkoutFailure(error: unknown): Failure {
    if (isSessionEnded(error)) return {text: messageFor(error), signIn: true};
    if (error instanceof ApiError) {
        if (error.code === "email_not_verified") {
            return {
                text: "Verify your email address before you start a trial.",
                link: {to: "/account", label: "Go to your account"},
            };
        }
        if (error.code === "already_subscribed") {
            return {
                text: "You already have a plan.",
                link: {to: "/dashboard/billing", label: "Open your billing page"},
            };
        }
        if (error.code === "billing_unavailable") {
            return {text: "Subscriptions are not available yet."};
        }
    }
    return {text: messageFor(error)};
}

type Phase = "checking" | "ready" | "signed_out" | "unavailable";

export default function SubscribePage() {
    const {ensureSession} = useAuth();
    const navigate = useNavigate();
    const location = useLocation();
    const [params] = useSearchParams();
    const cancelled = params.get("checkout") === "cancelled";

    const [plan, setPlan] = useState<PlanId>(() => parsePlan(params.get("plan")) ?? DEFAULT_PLAN);
    const [interval, setPeriod] = useState<Interval>(() => (params.get("interval") === "yearly" ? "yearly" : "monthly"));

    const [phase, setPhase] = useState<Phase>("checking");
    const [loadFailure, setLoadFailure] = useState<{ message: string; ended: boolean } | null>(null);
    const [attempt, setAttempt] = useState(0);
    const [busy, setBusy] = useState(false);
    const [failure, setFailure] = useState<Failure | null>(null);

    useEffect(() => {
        // StrictMode runs this twice. ensureSession shares one refresh, and the flag stops the first run from writing
        // state after the second has started.
        let active = true;
        setPhase("checking");
        setLoadFailure(null);
        (async () => {
            let user;
            try {
                user = await ensureSession();
            } catch (error) {
                if (!active) return;
                setLoadFailure({message: messageFor(error), ended: isSessionEnded(error)});
                setPhase("unavailable");
                return;
            }
            if (!active) return;
            if (!user) {
                setPhase("signed_out");
                return;
            }

            try {
                const subscription = await fetchSubscription();
                if (!active) return;
                if (hasPaidPlan(subscription)) {
                    navigate("/dashboard/billing", {replace: true});
                    return;
                }
            } catch (error) {
                if (!active) return;
                if (isSessionEnded(error)) {
                    setPhase("signed_out");
                    return;
                }
                // The plan check failed for another reason. The form still shows, and the checkout call enforces
                // already_subscribed itself.
            }
            if (active) setPhase("ready");
        })();
        return () => {
            active = false;
        };
    }, [ensureSession, navigate, attempt]);

    if (phase === "checking") {
        return (
            <PageShell wide>
                <p className="acct-status" role="status">Loading...</p>
            </PageShell>
        );
    }

    if (phase === "unavailable") {
        return (
            <PageShell wide>
                {loadFailure?.ended ? <SessionEndedNotice/> : <FormAlert message={loadFailure?.message ?? null}/>}
                <div className="acct-actions">
                    <button className="btn btn-primary acct-submit" type="button" onClick={() => setAttempt((n) => n + 1)}>
                        Try again
                    </button>
                </div>
            </PageShell>
        );
    }

    if (phase === "signed_out") {
        const next = encodeURIComponent(location.pathname + location.search);
        return <Navigate to={`/login?next=${next}`} replace/>;
    }

    async function onStart() {
        if (busy) return;
        setBusy(true);
        setFailure(null);
        try {
            const url = await startCheckout(plan, interval);
            // The page leaves for the hosted checkout, so the button stays disabled.
            window.location.assign(url);
        } catch (error) {
            setFailure(checkoutFailure(error));
            setBusy(false);
        }
    }

    return (
        <PageShell wide>
            <div className="bill-subscribe">
                <h1 className="acct-title">Choose a plan</h1>
                <p className="acct-body">
                    Sync your instances across your devices. The launcher stays free.
                </p>

                {cancelled && (
                    <p className="acct-status" role="status">Checkout was cancelled. Choose a plan to try again.</p>
                )}

                <div className="bill-toggle" role="group" aria-label="Billing interval">
                    <button
                        type="button"
                        className={interval === "monthly" ? "is-on" : ""}
                        aria-pressed={interval === "monthly"}
                        onClick={() => setPeriod("monthly")}
                    >
                        Monthly
                    </button>
                    <button
                        type="button"
                        className={interval === "yearly" ? "is-on" : ""}
                        aria-pressed={interval === "yearly"}
                        onClick={() => setPeriod("yearly")}
                    >
                        Yearly <span className="bill-save">Save about 20%</span>
                    </button>
                </div>

                <ul className="bill-plans">
                    {PLANS.map((item) => {
                        const selected = item.id === plan;
                        return (
                            <li key={item.id}>
                                <button
                                    type="button"
                                    className={`bill-plan bill-plan-${item.id}${selected ? " is-selected" : ""}`}
                                    aria-pressed={selected}
                                    onClick={() => setPlan(item.id)}
                                >
                                    <span className="bill-plan-name">{item.name}</span>
                                    <span className="bill-plan-price">
                                        {interval === "monthly" ? `${item.monthly}/mo` : `${item.yearly}/yr`}
                                    </span>
                                    <span className="bill-plan-billing">
                                        {interval === "monthly" ? "Billed monthly" : "Billed yearly"}
                                    </span>
                                    <ul className="bill-plan-allowances">
                                        <li>{item.storage}</li>
                                        <li>{item.devices}</li>
                                        <li>{item.sharing}</li>
                                    </ul>
                                </button>
                            </li>
                        );
                    })}
                </ul>

                <div className="bill-cta">
                    <p className="acct-hint">7-day free trial. A card is required.</p>
                    {failure && (failure.signIn ? (
                        <SessionEndedNotice/>
                    ) : (
                        <p className="acct-alert" role="alert">
                            {failure.text}
                            {failure.link && (
                                <>
                                    {" "}
                                    <Link to={failure.link.to}>{failure.link.label}</Link>
                                </>
                            )}
                        </p>
                    ))}
                    <button className="btn btn-primary acct-submit" type="button" onClick={onStart}
                            disabled={busy} aria-busy={busy}>
                        {busy && <span className="acct-spinner" aria-hidden="true"/>}
                        {busy ? "Starting..." : "Start free trial"}
                    </button>
                </div>
            </div>
        </PageShell>
    );
}
