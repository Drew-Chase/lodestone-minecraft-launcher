import {Link} from "react-router";
import {ApiError} from "../../api/errors.ts";

// Why adding friends is closed for this account. Shared by the friends and invite pages so the wording matches.
export type Gate = "email" | "username";

const GATE_TEXT: Readonly<Record<Gate, string>> = {
    email: "Verify your email to add friends",
    username: "Choose a username to add friends",
};

/** The gate an API error code means, or null when the code is not about the account. */
export function gateOfCode(code: string | null): Gate | null {
    if (code === "email_not_verified") return "email";
    if (code === "username_required") return "username";
    return null;
}

export function gateOfError(error: unknown): Gate | null {
    if (!(error instanceof ApiError)) return null;
    return gateOfCode(error.code);
}

export default function GateNotice({gate}: { gate: Gate }) {
    return (
        <p className="soc-gate" role="status">
            <span>{GATE_TEXT[gate]}.</span>{" "}
            <Link to="/account">Go to your account</Link>
        </p>
    );
}
