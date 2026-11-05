import type {ReactNode} from "react";
import {Link} from "react-router";

// The split layout for sign-in and the other account forms: a brand panel on the left and the form card on the right.
// Below the narrow breakpoint the two panels stack, with the brand panel above.
export default function AuthLayout({children}: { children: ReactNode }) {
    return (
        <div className="acct-shell">
            <aside className="acct-brand">
                <Link className="acct-brand-mark" to="/">
                    <img src="/lodestone-logo.svg" alt="" width={40} height={40}/>
                    <span className="acct-brand-name">Lodestone</span>
                </Link>
                <p className="acct-tagline">Every Minecraft instance, in one launcher.</p>
            </aside>
            <main className="acct-main">
                <div className="acct-card">{children}</div>
            </main>
        </div>
    );
}
