import {apiRequest} from "./client.ts";

// Shapes from the device endpoints. Keep these in step with the API.

export interface Device {
    id: number;
    name: string;
    os: string;
    app_version: string;
    created_at: string;
    last_seen_at: string | null;
}

export interface DeviceList {
    devices: Device[];
    /** null means unlimited. */
    limit: number | null;
}

export async function listDevices(): Promise<DeviceList> {
    return apiRequest<DeviceList>("/api/devices", {auth: true});
}

/** Approves the launcher sign-in that shows this code. Lowercase and a missing hyphen are accepted. */
export function approveDevice(userCode: string): Promise<{ status: "approved" }> {
    return apiRequest<{ status: "approved" }>("/api/device/approve", {
        method: "POST",
        body: {user_code: userCode},
        auth: true,
    });
}

/** What a pending sign-in code belongs to, shown before it is approved. */
export interface DeviceLookup {
    name: string;
    os: string;
    app_version: string;
    /** ISO time when the code stops working. */
    expires_at: string;
}

/** Reads the launcher sign-in behind this code without approving it. Lowercase and a missing hyphen are accepted. */
export function lookupDevice(userCode: string): Promise<DeviceLookup> {
    return apiRequest<DeviceLookup>("/api/device/lookup", {
        method: "POST",
        body: {user_code: userCode},
        auth: true,
    });
}

/** Removes a device. The device is signed out. */
export function removeDevice(id: number): Promise<void> {
    return apiRequest<void>(`/api/devices/${id}`, {method: "DELETE", auth: true});
}
