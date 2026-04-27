import {useCallback, useEffect, useRef, useState} from "react";
import {Button, Card} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import Chip from "../Chip";
import {I} from "../shell/icons";
import {cardSurfaceStyle, type Instance} from "../library/instances";

type Props = {
    instance: Instance;
};

type LogLevel = "INFO" | "WARN" | "ERROR" | "DEBUG" | "FATAL";

interface ParsedLine {
    time: string;
    level: LogLevel;
    text: string;
    raw: string;
}

const levelColor: Record<LogLevel, string> = {
    INFO: "var(--mc-green)",
    WARN: "var(--amber)",
    ERROR: "#ff5e5e",
    DEBUG: "var(--cyan)",
    FATAL: "#ff5e5e",
};

const levelRegex = /\[(\d{2}:\d{2}:\d{2})\]\s*\[([^\]]*\/(INFO|WARN|ERROR|DEBUG|FATAL))\]/;
const simpleLevelRegex = /\[(INFO|WARN|ERROR|DEBUG|FATAL)\]/;

function parseLine(raw: string): ParsedLine {
    // Try standard Minecraft log format: [HH:MM:SS] [Thread/LEVEL]
    const m = levelRegex.exec(raw);
    if (m) {
        return {time: m[1], level: m[3] as LogLevel, text: raw, raw};
    }
    // Try simpler format: [LEVEL]
    const s = simpleLevelRegex.exec(raw);
    if (s) {
        return {time: "", level: s[1] as LogLevel, text: raw, raw};
    }
    return {time: "", level: "INFO", text: raw, raw};
}

export default function LogsTab({instance}: Props) {
    const [logFiles, setLogFiles] = useState<string[]>([]);
    const [selectedFile, setSelectedFile] = useState("latest.log");
    const [content, setContent] = useState("");
    const scrollRef = useRef<HTMLDivElement>(null);

    const fetchLogFiles = useCallback(async () => {
        try {
            const files = await invoke<string[]>("list_log_files", {
                instancePath: instance.instancePath,
            });
            setLogFiles(files);
            if (files.length > 0 && !files.includes(selectedFile)) {
                setSelectedFile(files[0]);
            }
        } catch {
            setLogFiles([]);
        }
    }, [instance.instancePath]); // eslint-disable-line react-hooks/exhaustive-deps

    const fetchContent = useCallback(async () => {
        try {
            const text = await invoke<string>("read_log_file", {
                instancePath: instance.instancePath,
                fileName: selectedFile,
            });
            setContent(text);
        } catch {
            setContent("");
        }
    }, [instance.instancePath, selectedFile]);

    useEffect(() => {
        fetchLogFiles();
    }, [fetchLogFiles]);

    useEffect(() => {
        fetchContent();
    }, [fetchContent]);

    // Auto-scroll to bottom
    useEffect(() => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [content]);

    const handleOpenFolder = () => {
        const sep = instance.instancePath.includes("/") ? "/" : "\\";
        invoke("open_directory", {
            path: `${instance.instancePath}${sep}logs`,
        });
    };

    const handleCopy = () => {
        navigator.clipboard.writeText(content);
    };

    const lines = content.split("\n").filter((l) => l.length > 0);
    const parsed = lines.map(parseLine);

    return (
        <div className="flex-1 flex flex-col min-h-0 px-7 pt-5 pb-5">
            {logFiles.length === 0 ? (
                <div className="flex-1 flex items-center justify-center text-ink-3 text-sm">
                    <div className="text-center">
                        <I.terminal size={32} className="mx-auto mb-3 opacity-40"/>
                        <div>No log files found</div>
                        <div className="text-xs mt-1 text-ink-4">
                            Logs will appear after you launch the game
                        </div>
                    </div>
                </div>
            ) : (
                <Card
                    className="flex-1 min-h-0 flex flex-col p-0 border border-line font-mono text-[0.71875rem] leading-[1.7]"
                    style={cardSurfaceStyle}
                >
                    {/* File header */}
                    <div className="flex-shrink-0 flex items-center gap-2.5 px-4 py-2.5 border-b border-line">
                        <I.terminal size={14} className="text-mc-green"/>
                        <select
                            value={selectedFile}
                            onChange={(e) => setSelectedFile(e.target.value)}
                            className="text-xs font-semibold font-sans bg-transparent border border-line rounded-md px-2 py-1 text-ink-0 outline-none cursor-pointer"
                        >
                            {logFiles.map((f) => (
                                <option key={f} value={f}>{f}</option>
                            ))}
                        </select>
                        <div className="flex-1"/>
                        {selectedFile === "latest.log" && (
                            <Chip variant="green">
                                <span className="pulse-dot" style={{width: 5, height: 5}}/> LATEST
                            </Chip>
                        )}
                        <Button
                            variant="bordered"
                            size="sm"
                            className="text-[0.6875rem]"
                            startContent={<I.copy size={11}/>}
                            onPress={handleCopy}
                        >
                            Copy
                        </Button>
                        <Button
                            variant="bordered"
                            size="sm"
                            className="text-[0.6875rem]"
                            startContent={<I.folder size={11}/>}
                            onPress={handleOpenFolder}
                        >
                            Open Folder
                        </Button>
                        <Button
                            variant="bordered"
                            size="sm"
                            className="text-[0.6875rem]"
                            startContent={<I.refresh size={11}/>}
                            onPress={fetchContent}
                        >
                            Refresh
                        </Button>
                    </div>

                    {/* Log lines */}
                    <div
                        ref={scrollRef}
                        className="flex-1 min-h-0 overflow-y-auto px-4 py-2"
                    >
                        {parsed.map((l, i) => (
                            <div key={i} className="flex gap-3 hover:bg-[rgba(255,255,255,0.02)]">
                                {l.time && (
                                    <span className="text-ink-4 flex-shrink-0">{l.time}</span>
                                )}
                                <span
                                    className="flex-shrink-0 min-w-[50px]"
                                    style={{color: levelColor[l.level]}}
                                >
                                    [{l.level}]
                                </span>
                                <span className="text-ink-1 break-all">{l.text}</span>
                            </div>
                        ))}
                    </div>
                </Card>
            )}
        </div>
    );
}
