import {useEffect, useState} from "react";
import type {FileEntry} from "../../api/instances.ts";
import {formatModified, formatSize} from "./format.ts";
import {nameOf} from "./tree.ts";
import {previewKind, TEXT_PREVIEW_LIMIT} from "./preview.ts";

interface TextContent {
    /** The presigned address the text was fetched from. A different file means this content is stale. */
    url: string;
    status: "ready" | "failed";
    text: string;
}

interface FilePreviewProps {
    file: FileEntry | null;
}

export default function FilePreview({file}: FilePreviewProps) {
    if (!file) {
        return (
            <section className="fs-preview" aria-label="Preview">
                <p className="acct-body">Select a file to preview it.</p>
            </section>
        );
    }

    return (
        <section className="fs-preview" aria-label="Preview">
            <div className="fs-preview-head">
                <div className="fs-preview-info">
                    <span className="soc-name">{nameOf(file.path)}</span>
                    <span className="acct-hint">
                        {file.path} · {formatSize(file.size)} · Modified {formatModified(file.mtime_ms)}
                    </span>
                </div>
                <a className="btn btn-ghost" href={file.url} download={nameOf(file.path)}>Download</a>
            </div>
            <PreviewBody file={file}/>
        </section>
    );
}

function PreviewBody({file}: { file: FileEntry }) {
    const kind = previewKind(file.path);

    if (kind === "image") {
        // The browser loads the image from the presigned address directly, so the page fetches nothing.
        return <img className="fs-preview-image" src={file.url} alt={file.path}/>;
    }

    if (kind === "text") {
        if (file.size > TEXT_PREVIEW_LIMIT) {
            return <p className="acct-body">Too large to preview. Download the file to read it.</p>;
        }
        return <TextPreview file={file}/>;
    }

    return <p className="acct-body">No preview for this file type.</p>;
}

function TextPreview({file}: { file: FileEntry }) {
    const [content, setContent] = useState<TextContent | null>(null);

    useEffect(() => {
        const controller = new AbortController();
        // The presigned address needs no credentials, and the access token must not go to storage, so this is a plain
        // fetch rather than apiRequest.
        fetch(file.url, {credentials: "omit", signal: controller.signal})
            .then(async (response) => {
                if (!response.ok) throw new Error(`HTTP ${response.status}`);
                return response.text();
            })
            .then(
                (text) => setContent({url: file.url, status: "ready", text}),
                () => {
                    if (!controller.signal.aborted) setContent({url: file.url, status: "failed", text: ""});
                },
            );
        return () => controller.abort();
    }, [file.url]);

    if (content === null || content.url !== file.url) {
        return <p className="acct-status" role="status">Loading preview...</p>;
    }
    if (content.status === "failed") {
        return <p className="acct-alert" role="alert">Could not load this preview. Download the file instead.</p>;
    }
    // The text is rendered as a text node, so markup in the file is shown as written and never runs.
    return <pre className="fs-preview-text">{content.text}</pre>;
}
