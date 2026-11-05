import {useEffect, useState} from "react";
import {Link, Navigate, NavLink, Outlet, useLocation} from "react-router";
import {messageFor} from "../../api/errors.ts";
import {useAuth} from "../../auth/useAuth.ts";
import PageShell from "../auth/PageShell.tsx";
import FormAlert from "../auth/FormAlert.tsx";

export interface DashboardNavItem {
    to: string;
    label: string;
}

type Phase = "checking" | "ready" | "signed_out" | "unavailable";

// The signed-in shell for dashboard pages. The sidebar is driven by `items`, so a page is added to the list rather
// than to this file. Signed-out visitors go to sign-in, and come back to the page they asked for.
export default function DashboardLayout({items}: { items: readonly DashboardNavItem[] }) {
    const {ensureSession} = useAuth();
    const location = useLocation();
    const [phase, setPhase] = useState<Phase>("checking");
    const [failure, setFailure] = useState<string | null>(null);
    const [attempt, setAttempt] = useState(0);

    useEffect(() => {
        // StrictMode runs this twice. ensureSession shares one refresh between the runs, and the flag keeps the first
        // run from writing state after the second has started.
        let active = true;
        setPhase("checking");
        setFailure(null);
        ensureSession().then((user) => {
            if (active) setPhase(user ? "ready" : "signed_out");
        }, (error: unknown) => {
            if (!active) return;
            setFailure(messageFor(error));
            setPhase("unavailable");
        });
        return () => {
            active = false;
        };
    }, [ensureSession, attempt]);

    if (phase === "checking") {
        return (
            <PageShell>
                <p className="acct-status" role="status">Loading...</p>
            </PageShell>
        );
    }

    if (phase === "unavailable") {
        return (
            <PageShell>
                <FormAlert message={failure}/>
                <div className="acct-actions">
                    <button className="btn btn-primary acct-submit" type="button" onClick={() => setAttempt((n) => n + 1)}>
                        Try again
                    </button>
                </div>
            </PageShell>
        );
    }

    if (phase === "signed_out") {
        const next = encodeURIComponent(location.pathname + location.search);
        return <Navigate to={`/login?next=${next}`} replace/>;
    }

    return (
        <div className="dash-shell">
            <aside className="dash-sidebar">
                <Link className="acct-brand-mark dash-brand" to="/">
                    <img src="/lodestone-logo.svg" alt="" width={26} height={26}/>
                    <span className="acct-brand-name">Lodestone</span>
                </Link>
                <nav className="dash-nav" aria-label="Dashboard">
                    <ul className="dash-nav-list">
                        {items.map((item) => (
                            <li key={item.to}>
                                <NavLink
                                    className={({isActive}) => (isActive ? "dash-link is-active" : "dash-link")}
                                    to={item.to}
                                >
                                    {item.label}
                                </NavLink>
                            </li>
                        ))}
                    </ul>
                </nav>
            </aside>
            <main className="dash-main">
                <Outlet/>
            </main>
        </div>
    );
}
