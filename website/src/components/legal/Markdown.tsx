import type {ReactNode} from "react";

// Renders the markdown subset the legal pages use: # / ## / ### headings, paragraphs, "- " bullets, **bold**, `code`
// and [links](url). Everything else is plain text, so a document cannot inject markup. Links are limited to http(s),
// mailto and same-site paths.

const INLINE = /(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\([^)\s]+\))/;
const LINK = /^\[([^\]]+)\]\(([^)\s]+)\)$/;
const SAFE_HREF = /^(https?:\/\/|mailto:|\/(?!\/))/;

/** The inline pieces of one line. The split keeps each match at an odd index, so text and matches alternate. */
function inline(text: string, key: string): ReactNode[] {
    return text.split(INLINE).map((token, index) => {
        if (index % 2 === 0) {
            return token;
        }
        const id = `${key}-${index}`;
        if (token.startsWith("**")) {
            return <strong key={id}>{token.slice(2, -2)}</strong>;
        }
        if (token.startsWith("`")) {
            return <code key={id}>{token.slice(1, -1)}</code>;
        }
        const link = LINK.exec(token);
        if (link === null) {
            return token;
        }
        return SAFE_HREF.test(link[2]) ? <a key={id} href={link[2]}>{link[1]}</a> : link[1];
    });
}

/** A legal document: its markdown source, rendered as headings, paragraphs and lists. */
export default function Markdown({source}: {source: string}) {
    const blocks: ReactNode[] = [];
    let paragraph: string[] = [];
    let items: string[] = [];
    let count = 0;
    const nextKey = () => `b${count++}`;

    const flushParagraph = () => {
        if (paragraph.length === 0) {
            return;
        }
        const key = nextKey();
        blocks.push(<p key={key} className="acct-body">{inline(paragraph.join(" "), key)}</p>);
        paragraph = [];
    };

    const flushList = () => {
        if (items.length === 0) {
            return;
        }
        const key = nextKey();
        blocks.push(
            <ul key={key} className="legal-list">
                {items.map((item, index) => (
                    <li key={index} className="acct-body">{inline(item, `${key}-${index}`)}</li>
                ))}
            </ul>,
        );
        items = [];
    };

    const flush = () => {
        flushParagraph();
        flushList();
    };

    for (const raw of source.split("\n")) {
        const line = raw.trim();
        if (line === "") {
            flush();
            continue;
        }

        const heading = /^(#{1,3}) (.*)$/.exec(line);
        if (heading !== null) {
            flush();
            const key = nextKey();
            const text = inline(heading[2], key);
            if (heading[1] === "#") {
                blocks.push(<h1 key={key} className="acct-title">{text}</h1>);
            } else if (heading[1] === "##") {
                blocks.push(<h2 key={key} className="legal-h2">{text}</h2>);
            } else {
                blocks.push(<h3 key={key} className="legal-h3">{text}</h3>);
            }
            continue;
        }

        if (line.startsWith("- ")) {
            flushParagraph();
            items.push(line.slice(2));
            continue;
        }

        flushList();
        paragraph.push(line);
    }
    flush();

    return <>{blocks}</>;
}
