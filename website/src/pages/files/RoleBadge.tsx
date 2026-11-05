import type {Role} from "../../api/instances.ts";
import {ROLE_LABEL} from "./format.ts";

export default function RoleBadge({role}: { role: Role }) {
    return <span className={`fs-badge fs-badge-${role}`}>{ROLE_LABEL[role]}</span>;
}
