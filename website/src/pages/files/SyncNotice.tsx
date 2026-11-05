import {Link} from "react-router";
import {ApiError} from "../../api/errors.ts";

// Why sync is closed for this account. The notice is local to the files pages because the shared GateNotice only
// covers the friends gates, but it uses the same markup and classes.
export type SyncGate = "email" | "plan";

const GATE_TEXT: Readonly<Record<SyncGate, string>> = {
    email: "Verify your email to sync",
    plan: "Sync needs a plan",
};

/** The gate an API error means for sync, or null when the error is about something else. */
export function syncGateOf(error: unknown): SyncGate | null {
    if (!(error instanceof ApiError)) return null;
    if (error.code === "email_not_verified") return "email";
    if (error.code === "plan_required") return "plan";
    return null;
}

export default function SyncNotice({gate}: { gate: SyncGate }) {
    return (
        <p className="soc-gate" role="status">
            <span>{GATE_TEXT[gate]}.</span>{" "}
            {gate === "email"
                ? <Link to="/account">Go to your account</Link>
                : <Link to="/subscribe">Choose a plan</Link>}
        </p>
    );
}
