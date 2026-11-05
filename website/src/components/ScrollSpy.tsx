import {useEffect, useState} from "react";

// The sections on the page, in order. Each id is the section's id in the page.
const SECTIONS = [
    {id: "top", label: "Top"},
    {id: "features", label: "Features"},
    {id: "showcase", label: "Deep dive"},
    {id: "how", label: "How it works"},
    {id: "pricing", label: "Pricing"},
    {id: "open-source", label: "Open source"},
];

// The reading line sits this far down the window. A section is current once its top has passed the line.
const READING_LINE = 0.4;
// The timeline appears once the reader has scrolled this many window heights.
const SHOW_AFTER = 0.5;

/**
 * Where the reading line is, as a position along the timeline. A whole number is a section's top, and a fraction is
 * how far the line has travelled toward the next section, so the fill moves smoothly between the nodes.
 */
function locate(): number {
    const line = window.scrollY + window.innerHeight * READING_LINE;
    const tops = SECTIONS.map(({id}) => {
        const el = document.getElementById(id);
        return el ? el.getBoundingClientRect().top + window.scrollY : Infinity;
    });

    let index = 0;
    tops.forEach((top, i) => {
        if (top <= line) index = i;
    });

    const next = tops[index + 1];
    if (next === undefined || next === Infinity) return index;
    const between = (line - tops[index]) / (next - tops[index]);
    return index + Math.min(1, Math.max(0, between));
}

export default function ScrollSpy() {
    const [position, setPosition] = useState(0);
    const [shown, setShown] = useState(false);

    useEffect(() => {
        let frame = 0;
        const update = () => {
            frame = 0;
            setPosition(locate());
            setShown(window.scrollY > window.innerHeight * SHOW_AFTER);
        };
        const schedule = () => {
            if (!frame) frame = window.requestAnimationFrame(update);
        };

        update();
        window.addEventListener("scroll", schedule, {passive: true});
        window.addEventListener("resize", schedule);
        return () => {
            window.removeEventListener("scroll", schedule);
            window.removeEventListener("resize", schedule);
            window.cancelAnimationFrame(frame);
        };
    }, []);

    const last = SECTIONS.length - 1;
    const current = Math.floor(position);

    return (
        <nav className={`timeline${shown ? " on" : ""}`} aria-label="Page sections">
            <div className="tl-track">
                <div className="tl-rail">
                    <div className="tl-fill" style={{transform: `scaleY(${position / last})`}}/>
                </div>
                {SECTIONS.map((section, i) => (
                    <a key={section.id} href={`#${section.id}`}
                       className={`tl-node${i < current ? " past" : ""}${i === current ? " current" : ""}`}
                       style={{top: `${(i / last) * 100}%`}}
                       aria-current={i === current ? "location" : undefined}>
                        <span className="tl-dot"/>
                        <span className="tl-label">{section.label}</span>
                    </a>
                ))}
            </div>
        </nav>
    );
}
