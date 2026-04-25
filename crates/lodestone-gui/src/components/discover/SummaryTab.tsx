import Markdown from "react-markdown";
import rehypeRaw from "rehype-raw";
import rehypeSanitize from "rehype-sanitize";
import {cardSurfaceStyle} from "../surfaces";

export default function SummaryTab({description, summary}: {description: string | null; summary: string}) {
    const content = description || summary;

    return (
        <div
            className="border border-line rounded-xl summary-content"
            style={{...cardSurfaceStyle, padding: 22}}
        >
            <div
                style={{
                    fontSize: 13,
                    lineHeight: 1.7,
                    color: "var(--ink-1)",
                    wordBreak: "break-word",
                }}
            >
                <Markdown
                    rehypePlugins={[rehypeRaw, rehypeSanitize]}
                    components={{
                        a: ({children, href, ...props}) => (
                            <a
                                href={href}
                                target="_blank"
                                rel="noopener noreferrer"
                                style={{color: "var(--mc-green)", textDecoration: "underline"}}
                                {...props}
                            >
                                {children}
                            </a>
                        ),
                        img: ({src, alt, ...props}) => (
                            <img
                                src={src}
                                alt={alt ?? ""}
                                style={{maxWidth: "100%", borderRadius: 8, margin: "8px 0"}}
                                loading="lazy"
                                {...props}
                            />
                        ),
                        h1: ({children, ...props}) => (
                            <h1 style={{fontSize: 22, fontWeight: 700, margin: "16px 0 8px", color: "#fff"}} {...props}>{children}</h1>
                        ),
                        h2: ({children, ...props}) => (
                            <h2 style={{fontSize: 18, fontWeight: 700, margin: "14px 0 6px", color: "#fff"}} {...props}>{children}</h2>
                        ),
                        h3: ({children, ...props}) => (
                            <h3 style={{fontSize: 15, fontWeight: 700, margin: "12px 0 4px", color: "#fff"}} {...props}>{children}</h3>
                        ),
                        p: ({children, ...props}) => (
                            <p style={{margin: "8px 0"}} {...props}>{children}</p>
                        ),
                        ul: ({children, ...props}) => (
                            <ul style={{paddingLeft: 20, margin: "8px 0"}} {...props}>{children}</ul>
                        ),
                        ol: ({children, ...props}) => (
                            <ol style={{paddingLeft: 20, margin: "8px 0"}} {...props}>{children}</ol>
                        ),
                        li: ({children, ...props}) => (
                            <li style={{margin: "2px 0"}} {...props}>{children}</li>
                        ),
                        blockquote: ({children, ...props}) => (
                            <blockquote
                                style={{
                                    borderLeft: "3px solid var(--mc-green)",
                                    paddingLeft: 12,
                                    margin: "8px 0",
                                    color: "var(--ink-2)",
                                }}
                                {...props}
                            >
                                {children}
                            </blockquote>
                        ),
                        code: ({children, className, ...props}) => {
                            const isBlock = className?.includes("language-");
                            if (isBlock) {
                                return (
                                    <pre
                                        style={{
                                            background: "rgba(0,0,0,0.3)",
                                            borderRadius: 8,
                                            padding: 12,
                                            margin: "8px 0",
                                            overflowX: "auto",
                                            fontSize: 12,
                                            fontFamily: "var(--mono)",
                                        }}
                                    >
                                        <code {...props}>{children}</code>
                                    </pre>
                                );
                            }
                            return (
                                <code
                                    style={{
                                        background: "rgba(255,255,255,0.06)",
                                        padding: "2px 5px",
                                        borderRadius: 4,
                                        fontSize: 12,
                                        fontFamily: "var(--mono)",
                                    }}
                                    {...props}
                                >
                                    {children}
                                </code>
                            );
                        },
                        hr: (props) => (
                            <hr style={{border: "none", borderTop: "1px solid var(--line)", margin: "16px 0"}} {...props}/>
                        ),
                        table: ({children, ...props}) => (
                            <div style={{overflowX: "auto", margin: "8px 0"}}>
                                <table
                                    style={{
                                        width: "100%",
                                        borderCollapse: "collapse",
                                        fontSize: 12,
                                    }}
                                    {...props}
                                >
                                    {children}
                                </table>
                            </div>
                        ),
                        th: ({children, ...props}) => (
                            <th
                                style={{
                                    textAlign: "left",
                                    padding: "6px 10px",
                                    borderBottom: "1px solid var(--line)",
                                    fontWeight: 700,
                                    color: "var(--ink-2)",
                                }}
                                {...props}
                            >
                                {children}
                            </th>
                        ),
                        td: ({children, ...props}) => (
                            <td
                                style={{
                                    padding: "6px 10px",
                                    borderBottom: "1px solid var(--line)",
                                }}
                                {...props}
                            >
                                {children}
                            </td>
                        ),
                    }}
                >
                    {content}
                </Markdown>
            </div>
        </div>
    );
}
