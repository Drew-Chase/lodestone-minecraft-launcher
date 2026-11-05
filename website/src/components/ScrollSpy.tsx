import {useEffect, useRef, useState} from "react";
import type {MouseEvent, PointerEvent} from "react";

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
function sectionTops(): number[] {
    return SECTIONS.map(({id}) => {
        const el = document.getElementById(id);
        return el ? el.getBoundingClientRect().top + window.scrollY : Infinity;
    });
}

function locate(): number {
    const line = window.scrollY + window.innerHeight * READING_LINE;
    const tops = sectionTops();

    let index = 0;
    tops.forEach((top, i) => {
        if (top <= line) index = i;
    });

    const next = tops[index + 1];
    if (next === undefined || next === Infinity) return index;
    const between = (line - tops[index]) / (next - tops[index]);
    return index + Math.min(1, Math.max(0, between));
}

/** The page offset that puts a timeline position on the reading line. It is the inverse of locate(), clamped to the page. */
function scrollFor(position: number): number {
    const tops = sectionTops();
    const i = Math.min(Math.floor(position), tops.length - 2);
    const top = tops[i] + (position - i) * (tops[i + 1] - tops[i]);
    const max = document.documentElement.scrollHeight - window.innerHeight;
    return Math.min(max, Math.max(0, top - window.innerHeight * READING_LINE));
}

export default function ScrollSpy() {
    const [position, setPosition] = useState(0);
    const [shown, setShown] = useState(false);
    const [dragging, setDragging] = useState(false);
    const track = useRef<HTMLDivElement>(null);

    // Scrolls so the pointer's height on the rail is the matching place on the page, the way a scrollbar thumb works.
    const seek = (clientY: number, behavior: ScrollBehavior) => {
        const rect = track.current?.getBoundingClientRect();
        if (!rect) return;
        const fraction = Math.min(1, Math.max(0, (clientY - rect.top) / rect.height));
        window.scrollTo({top: scrollFor(fraction * (SECTIONS.length - 1)), behavior});
    };

    const onPointerDown = (e: PointerEvent<HTMLElement>) => {
        if (e.button !== 0) return;
        e.currentTarget.setPointerCapture(e.pointerId);
        setDragging(true);
        seek(e.clientY, "instant");
    };

    const onPointerMove = (e: PointerEvent<HTMLElement>) => {
        if (dragging) seek(e.clientY, "instant");
    };

    const onPointerUp = (e: PointerEvent<HTMLElement>) => {
        if (!dragging) return;
        e.currentTarget.releasePointerCapture(e.pointerId);
        setDragging(false);
    };

    // A mouse or touch press has already scrolled on pointer down, so the link's own jump is cancelled. A keyboard
    // activation has no pointer, so it scrolls smoothly to its section.
    const jumpTo = (e: MouseEvent<HTMLAnchorElement>, index: number) => {
        e.preventDefault();
        if (e.detail === 0) window.scrollTo({top: scrollFor(index), behavior: "smooth"});
    };

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
        <nav className={`timeline${shown ? " on" : ""}${dragging ? " dragging" : ""}`} aria-label="Page sections"
             onPointerDown={onPointerDown} onPointerMove={onPointerMove}
             onPointerUp={onPointerUp} onPointerCancel={onPointerUp}>
            <div className="tl-track" ref={track}>
                <div className="tl-rail">
                    <div className="tl-fill" style={{transform: `scaleY(${position / last})`}}/>
                </div>
                {SECTIONS.map((section, i) => (
                    <a key={section.id} href={`#${section.id}`} onClick={e => jumpTo(e, i)}
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
