import {useEffect, useState} from "react";
import {Link} from "react-router";
import FormAlert from "../../components/auth/FormAlert.tsx";
import {listDevices, removeDevice} from "../../api/devices.ts";
import type {Device, DeviceList} from "../../api/devices.ts";
import {messageFor} from "../../api/errors.ts";
import {formatWhen} from "../social/format.ts";

// The count line. A null limit means the plan has no device cap.
function usageLine(count: number, limit: number | null): string {
    if (limit === null) return "Unlimited devices";
    return `Using ${count} of ${limit} devices`;
}

export default function DevicesPage() {
    const [data, setData] = useState<DeviceList | null>(null);
    const [loadFailure, setLoadFailure] = useState<string | null>(null);
    const [attempt, setAttempt] = useState(0);

    useEffect(() => {
        let active = true;
        setLoadFailure(null);
        listDevices().then((list) => {
            if (active) setData(list);
        }, (error: unknown) => {
            if (active) setLoadFailure(messageFor(error));
        });
        return () => {
            active = false;
        };
    }, [attempt]);

    if (loadFailure) {
        return (
            <div className="soc-page">
                <h1 className="acct-title">Devices</h1>
                <FormAlert message={loadFailure}/>
                <div className="acct-actions">
                    <button className="btn btn-primary acct-submit" type="button" onClick={() => setAttempt((n) => n + 1)}>
                        Try again
                    </button>
                </div>
            </div>
        );
    }

    if (!data) {
        return (
            <div className="soc-page">
                <h1 className="acct-title">Devices</h1>
                <p className="acct-status" role="status">Loading your devices...</p>
            </div>
        );
    }

    return <DevicesContent list={data} onChanged={setData}/>;
}

function DevicesContent({list, onChanged}: { list: DeviceList; onChanged: (list: DeviceList) => void }) {
    const [confirming, setConfirming] = useState<number | null>(null);
    const [busyId, setBusyId] = useState<number | null>(null);
    const [error, setError] = useState<string | null>(null);

    const count = list.devices.length;
    const atLimit = list.limit !== null && count >= list.limit;

    async function onRemove(id: number) {
        if (busyId !== null) return;
        setBusyId(id);
        setError(null);
        try {
            await removeDevice(id);
            setConfirming(null);
            onChanged(await listDevices());
        } catch (err) {
            setError(messageFor(err, {not_found: "That device is no longer on your account."}));
            setConfirming(null);
        } finally {
            setBusyId(null);
        }
    }

    return (
        <div className="soc-page">
            <h1 className="acct-title">Devices</h1>

            <section className="acct-section" aria-labelledby="soc-usage-title">
                <h2 className="acct-subtitle" id="soc-usage-title">Signed-in devices</h2>
                <p className="soc-usage">{usageLine(count, list.limit)}</p>
                <p className="acct-body">Removing a device signs it out.</p>
                {atLimit && (
                    <p className="acct-body">
                        You have reached your device limit. <Link to="/dashboard/billing">Change your plan</Link> to add more.
                    </p>
                )}
                <FormAlert message={error}/>
            </section>

            <section className="acct-section" aria-labelledby="soc-devices-title">
                <h2 className="acct-subtitle" id="soc-devices-title">Devices</h2>
                {count === 0 && <p className="acct-body">No devices are signed in.</p>}
                {count > 0 && (
                    <ul className="soc-list">
                        {list.devices.map((device) => (
                            <DeviceRow
                                key={device.id}
                                device={device}
                                confirming={confirming === device.id}
                                busy={busyId === device.id}
                                disabled={busyId !== null}
                                onAsk={() => setConfirming(device.id)}
                                onCancel={() => setConfirming(null)}
                                onConfirm={() => onRemove(device.id)}
                            />
                        ))}
                    </ul>
                )}
            </section>
        </div>
    );
}

interface DeviceRowProps {
    device: Device;
    confirming: boolean;
    busy: boolean;
    disabled: boolean;
    onAsk: () => void;
    onCancel: () => void;
    onConfirm: () => void;
}

function DeviceRow({device, confirming, busy, disabled, onAsk, onCancel, onConfirm}: DeviceRowProps) {
    const lastSeen = device.last_seen_at ? formatWhen(device.last_seen_at) : "Never";

    return (
        <li className="soc-device">
            <div className="soc-device-info">
                <span className="soc-device-name">{device.name}</span>
                <span className="acct-hint">{device.os} · Launcher {device.app_version}</span>
                <span className="acct-hint">First signed in {formatWhen(device.created_at)}</span>
                <span className="acct-hint">Last seen {lastSeen}</span>
            </div>
            <div className="soc-row-actions">
                {confirming ? (
                    <>
                        <button className="btn btn-primary soc-action" type="button" onClick={onConfirm} disabled={busy}>
                            {busy ? "Removing..." : "Confirm remove"}
                        </button>
                        <button className="btn btn-ghost soc-action" type="button" onClick={onCancel} disabled={busy}>
                            Cancel
                        </button>
                    </>
                ) : (
                    <button className="btn btn-ghost soc-action" type="button" onClick={onAsk} disabled={disabled}
                            aria-label={`Remove ${device.name}`}>
                        Remove
                    </button>
                )}
            </div>
        </li>
    );
}
