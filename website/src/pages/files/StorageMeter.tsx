import {Link} from "react-router";
import FormAlert from "../../components/auth/FormAlert.tsx";
import type {Storage} from "../../api/storage.ts";
import {formatGigabytes} from "./format.ts";

export default function StorageMeter({storage, failure}: { storage: Storage | null; failure: string | null }) {
    return (
        <section className="acct-section" aria-labelledby="fs-storage-title">
            <h2 className="acct-subtitle" id="fs-storage-title">Storage</h2>
            <FormAlert message={failure}/>
            {storage === null && failure === null && <p className="acct-status" role="status">Loading storage...</p>}
            {storage !== null && <StorageDetail storage={storage}/>}
        </section>
    );
}

function StorageDetail({storage}: { storage: Storage }) {
    const used = formatGigabytes(storage.used_bytes);

    // A zero limit means the account has no plan, so there is nothing to measure against.
    if (storage.limit_bytes === 0) {
        return (
            <>
                <p className="soc-usage">No plan</p>
                <p className="acct-body">Using {used}. Choose a plan to sync instances.</p>
                <div className="acct-actions">
                    <Link className="btn btn-primary" to="/subscribe">Choose a plan</Link>
                </div>
            </>
        );
    }

    const limit = formatGigabytes(storage.limit_bytes);
    const percent = Math.min(100, (storage.used_bytes / storage.limit_bytes) * 100);
    return (
        <>
            <p className="soc-usage">Used {used} of {limit}</p>
            <div
                className="fs-meter"
                role="meter"
                aria-label="Storage used"
                aria-valuemin={0}
                aria-valuemax={storage.limit_bytes}
                aria-valuenow={Math.min(storage.used_bytes, storage.limit_bytes)}
                aria-valuetext={`Used ${used} of ${limit}`}
            >
                <span className="fs-meter-fill" style={{width: `${percent}%`}}/>
            </div>
            {storage.name && <p className="acct-hint">{storage.name} plan</p>}
        </>
    );
}
