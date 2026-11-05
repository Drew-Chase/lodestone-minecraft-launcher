import Logo from "./Logo.tsx";

// A fixed, transparent bar that carries only the logo and the title. The bar lets clicks through, so only the brand
// takes them.
export default function Navigation() {
    return (
        <nav className="nav">
            <div className="container nav-inner">
                <a className="brand" href="#top">
                    <span className="brand-mark"><Logo size={26}/></span>
                    Lodestone
                </a>
            </div>
        </nav>
    );
}
