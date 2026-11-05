import type {ReactNode} from "react";
import {Link} from "react-router";

// A plain page on the dark background, with a small brand bar and one centred column. The account, legal and
// not-found pages use it.
export default function PageShell({children, wide = false}: { children: ReactNode; wide?: boolean }) {
    return (
        <div className="acct-page">
            <header className="acct-topbar">
                <div className="acct-topbar-inner">
                    <Link className="acct-brand-mark" to="/">
                        <img src="/lodestone-logo.svg" alt="" width={26} height={26}/>
                        <span className="acct-brand-name">Lodestone</span>
                    </Link>
                </div>
            </header>
            <main className={wide ? "acct-content acct-content-wide" : "acct-content"}>{children}</main>
        </div>
    );
}
