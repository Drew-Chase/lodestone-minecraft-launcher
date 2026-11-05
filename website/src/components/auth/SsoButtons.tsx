import {useEffect, useState} from "react";
import {fetchProviders} from "../../api/providers.ts";
import type {ProviderName} from "../../api/providers.ts";

const LABELS: Readonly<Record<ProviderName, string>> = {
    google: "Continue with Google",
    microsoft: "Continue with Microsoft",
    github: "Continue with GitHub",
};

// The single sign-on buttons. They are plain links, so the browser makes a full navigation to the provider's start
// endpoint. Nothing renders while the provider list loads, and nothing when no provider is configured.
export default function SsoButtons({next = "/account"}: { next?: string }) {
    const [providers, setProviders] = useState<ProviderName[] | null>(null);

    useEffect(() => {
        let cancelled = false;
        fetchProviders().then((list) => {
            if (!cancelled) setProviders(list);
        });
        return () => {
            cancelled = true;
        };
    }, []);

    if (!providers) return null;
    const enabled = providers.filter((provider) => provider in LABELS);
    if (enabled.length === 0) return null;

    return (
        <div className="sso">
            <div className="sso-buttons">
                {enabled.map((provider) => (
                    <a
                        key={provider}
                        className={`sso-button sso-${provider}`}
                        href={`/api/auth/${provider}/start?next=${encodeURIComponent(next)}`}
                    >
                        {LABELS[provider]}
                    </a>
                ))}
            </div>
            <div className="sso-divider" role="separator">
                <span>or</span>
            </div>
        </div>
    );
}
