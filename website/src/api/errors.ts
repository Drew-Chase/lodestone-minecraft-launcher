// An error from the API, or a failure to reach it. `code` is one of the codes in api/README.md, or `network_error`
// when the request never got a response (status 0).
export class ApiError extends Error {
    readonly code: string;
    readonly status: number;
    /** Seconds to wait, from the Retry-After header, when the server sent one. */
    readonly retryAfter: number | undefined;

    constructor(code: string, status: number, retryAfter?: number) {
        super(code);
        this.name = "ApiError";
        this.code = code;
        this.status = status;
        this.retryAfter = retryAfter;
    }
}

const FALLBACK = "Something went wrong. Try again.";

const MESSAGES: Readonly<Record<string, string>> = {
    invalid_credentials: "Email or password is not right.",
    missing_field: "Fill in every field.",
    invalid_email: "Enter a valid email address.",
    invalid_display_name: "Use 64 characters or fewer.",
    password_too_short: "Use at least 12 characters.",
    password_too_long: "That password is too long.",
    password_breached: "That password appears in a data breach. Choose another.",
    invalid_token: "This link is no longer valid. Request a new one.",
    token_expired: "This link is no longer valid. Request a new one.",
    rate_limited: "Too many attempts. Try again in a few minutes.",
    refresh_invalid: "Your session has ended. Sign in again.",
    refresh_reused: "Your session has ended. Sign in again.",
    refresh_expired: "Your session has ended. Sign in again.",
    unauthorized: "Your session has ended. Sign in again.",
    network_error: "Cannot reach Lodestone. Check your connection.",
    invalid_username: "Use 3 to 16 letters, digits or underscores.",
    username_taken: "That username is taken. Choose another.",
    invalid_code: "That code was not found. Check it and try again.",
    code_expired: "That code has expired. Start sign-in again in the launcher.",
    code_used: "That code was already used.",
    device_limit_reached: "This account has reached its device limit. Remove a device, then try again.",
    not_found: "No one matched that. Check it and try again.",
    invalid_friend: "You cannot add that account as a friend.",
    already_friends: "You are already friends.",
    request_pending: "A friend request is already waiting.",
    email_not_verified: "Verify your email address first.",
    username_required: "Choose a username first.",
    invalid_invite: "This invite link is not valid.",
    invite_expired: "This invite link has expired. Ask for a new one.",
    invite_used: "This invite link has already been used.",
    minecraft_not_owned: "That Microsoft account does not own Minecraft.",
    minecraft_restricted: "Xbox Live restricts this account in its country, region or age settings, or it is a child account that is not yet in a family.",
    minecraft_linked_elsewhere: "That Minecraft account is linked to another Lodestone account.",
    minecraft_unavailable: "Minecraft services are unavailable. Try again later.",
    slow_down: "Too many checks. Wait a moment and try again.",
    already_subscribed: "You already have a plan.",
    billing_unavailable: "Subscriptions are not available yet.",
    billing_failed: "Billing failed. Try again.",
    invalid_plan: "That plan is not available. Choose another plan.",
    no_billing_account: "No billing account yet.",
    storage_unavailable: "Storage is not available right now. Try again later.",
    quota_exceeded: "This would go over your storage limit. Free up space or change your plan.",
    plan_required: "Sync needs a plan. Choose one to continue.",
    upload_incomplete: "The upload did not finish. Try again.",
    upload_expired: "The upload took too long and expired. Try again.",
    upload_committed: "This upload was already saved.",
    sharing_not_allowed: "Sharing needs a plan that allows it.",
    share_limit_reached: "You have reached the number of instances you can share.",
    invalid_share: "You cannot share with that account.",
    invalid_access: "Choose viewer or editor.",
    invalid_name: "That name is not allowed. Choose another.",
    invalid_manifest: "This version could not be read. Try again.",
    forbidden: "You do not have permission to do that.",
};

/** The copy for one API error code. */
export function messageForCode(code: string): string {
    return MESSAGES[code] ?? FALLBACK;
}

/**
 * The copy to show for a failed call. `overrides` replaces a code's copy where the form needs its own wording, such as
 * "current password" rather than "email or password".
 */
export function messageFor(error: unknown, overrides?: Readonly<Record<string, string>>): string {
    if (error instanceof ApiError) {
        return overrides?.[error.code] ?? messageForCode(error.code);
    }
    return FALLBACK;
}
