import {Link, useLocation} from "react-router";

// Shown when the session has ended. The link sends the visitor back to this page once they sign in again.
export default function SessionEndedNotice() {
    const location = useLocation();
    const next = encodeURIComponent(location.pathname + location.search);
    return (
        <p className="acct-alert" role="alert">
            Your session has ended. <Link to={`/login?next=${next}`}>Sign in again.</Link>
        </p>
    );
}
