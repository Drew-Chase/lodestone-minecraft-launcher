import PageShell from "../auth/PageShell.tsx";

interface LegalPageProps {
    title: string;
    paragraphs: string[];
}

// A legal page with its placeholder notice. The final text replaces the paragraphs passed in.
export default function LegalPage({title, paragraphs}: LegalPageProps) {
    return (
        <PageShell>
            <article className="acct-legal">
                <h1 className="acct-title">{title}</h1>
                <p className="acct-notice" role="note">Placeholder: the final text will replace this.</p>
                {paragraphs.map((text) => <p key={text} className="acct-body">{text}</p>)}
            </article>
        </PageShell>
    );
}
