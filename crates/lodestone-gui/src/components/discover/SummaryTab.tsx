import {cardSurfaceStyle} from "../surfaces";

export default function SummaryTab({description, summary}: {description: string | null; summary: string}) {
    const html = description || summary;

    return (
        <div
            className="border border-line rounded-xl"
            style={{...cardSurfaceStyle, padding: 22}}
        >
            <div
                style={{
                    fontSize: 13,
                    lineHeight: 1.7,
                    color: "var(--ink-1)",
                    wordBreak: "break-word",
                }}
                dangerouslySetInnerHTML={description ? {__html: html} : undefined}
            >
                {!description ? html : undefined}
            </div>
        </div>
    );
}
