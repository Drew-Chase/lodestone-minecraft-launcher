import {Link} from "react-router";
import PageShell from "../components/auth/PageShell.tsx";

export default function NotFoundPage() {
    return (
        <PageShell>
            <article className="acct-legal">
                <h1 className="acct-title">Page not found</h1>
                <p className="acct-body">The page you asked for does not exist.</p>
                <div className="acct-actions">
                    <Link className="btn btn-ghost acct-submit" to="/">Back to Lodestone</Link>
                </div>
            </article>
        </PageShell>
    );
}
