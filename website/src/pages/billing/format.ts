import type {Subscription} from "../../api/billing.ts";

/** The API counts storage in decimal gigabytes: 1 GB is 1,000,000,000 bytes. */
const GB = 1e9;

/** "100 GB storage" */
export function formatStorage(bytes: number): string {
    return `${Math.round(bytes / GB)} GB storage`;
}

export function formatDevices(devices: Subscription["devices"]): string {
    if (devices === null) return "Unlimited devices";
    return devices === 1 ? "1 device" : `${devices} devices`;
}

export function formatSharing(shares: Subscription["shares"]): string {
    if (shares === null) return "Unlimited sharing";
    if (shares === 0) return "No sharing";
    return `Share up to ${shares} instances`;
}

export function formatDate(value: string): string {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return date.toLocaleDateString(undefined, {year: "numeric", month: "long", day: "numeric"});
}
