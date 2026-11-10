import PageShell from "../auth/PageShell.tsx";
import Markdown from "./Markdown.tsx";

interface LegalPageProps {
    markdown: string;
}

// A legal page. Its text is a markdown file in src/content/legal, so the page and the file cannot drift apart.
export default function LegalPage({markdown}: LegalPageProps) {
    return (
        <PageShell>
            <article className="acct-legal">
                <Markdown source={markdown}/>
            </article>
        </PageShell>
    );
}
