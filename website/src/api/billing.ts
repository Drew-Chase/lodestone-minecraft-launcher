import {apiRequest} from "./client.ts";

// Shapes from the billing endpoints. Keep these in step with the API.

export type PlanId = "solo" | "friends" | "unlimited";
export type Interval = "monthly" | "yearly";

export type SubscriptionStatus =
    | "none"
    | "trialing"
    | "active"
    | "past_due"
    | "canceled"
    | "incomplete"
    | "incomplete_expired"
    | "unpaid"
    | "paused";

export interface Subscription {
    plan: "free" | PlanId;
    name: string;
    status: SubscriptionStatus;
    interval: Interval | null;
    storage_bytes: number;
    /** null means unlimited. */
    devices: number | null;
    /** null means unlimited. 0 means no sharing. */
    shares: number | null;
    trial_ends_at: string | null;
    current_period_end: string | null;
    cancel_at_period_end: boolean;
}

export async function fetchSubscription(): Promise<Subscription> {
    const data = await apiRequest<{ subscription: Subscription }>("/api/billing/subscription", {auth: true});
    return data.subscription;
}

/** Starts a checkout session and returns the hosted page's address. */
export async function startCheckout(plan: PlanId, interval: Interval): Promise<string> {
    const data = await apiRequest<{ url: string }>("/api/billing/checkout", {
        method: "POST",
        body: {plan, interval},
        auth: true,
    });
    return data.url;
}

/** Opens the billing portal session and returns its address. */
export async function openBillingPortal(): Promise<string> {
    const data = await apiRequest<{ url: string }>("/api/billing/portal", {method: "POST", auth: true});
    return data.url;
}
