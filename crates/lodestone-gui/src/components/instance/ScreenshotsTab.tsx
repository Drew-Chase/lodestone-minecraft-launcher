import {useCallback, useEffect, useState} from "react";
import {Button, Card} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {I} from "../shell/icons";
import {cardSurfaceStyle, type Instance} from "../library/instances";

type Props = {
    instance: Instance;
};

interface ScreenshotEntry {
    fileName: string;
    fileSizeBytes: number;
    lastModified: string;
}

function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function ScreenshotsTab({instance}: Props) {
    const [screenshots, setScreenshots] = useState<ScreenshotEntry[]>([]);
    const [imageCache, setImageCache] = useState<Record<string, string>>({});

    const fetchScreenshots = useCallback(async () => {
        try {
            const result = await invoke<ScreenshotEntry[]>("list_instance_screenshots", {
                instancePath: instance.instancePath,
            });
            setScreenshots(result);
        } catch {
            setScreenshots([]);
        }
    }, [instance.instancePath]);

    useEffect(() => {
        fetchScreenshots();
    }, [fetchScreenshots]);

    // Load screenshot images as base64 data URLs
    useEffect(() => {
        for (const ss of screenshots) {
            if (imageCache[ss.fileName]) continue;
            invoke<number[]>("read_instance_screenshot", {
                instancePath: instance.instancePath,
                fileName: ss.fileName,
            }).then((bytes) => {
                const uint8 = new Uint8Array(bytes);
                const blob = new Blob([uint8], {type: "image/png"});
                const url = URL.createObjectURL(blob);
                setImageCache((prev) => ({...prev, [ss.fileName]: url}));
            }).catch(() => {});
        }
    }, [screenshots, instance.instancePath]); // eslint-disable-line react-hooks/exhaustive-deps

    const handleDelete = async (fileName: string) => {
        if (!confirm(`Delete screenshot "${fileName}"?`)) return;
        await invoke("delete_screenshot", {
            instancePath: instance.instancePath,
            fileName,
        });
        // Clean up blob URL
        if (imageCache[fileName]) {
            URL.revokeObjectURL(imageCache[fileName]);
            setImageCache((prev) => {
                const next = {...prev};
                delete next[fileName];
                return next;
            });
        }
        fetchScreenshots();
    };

    const handleOpenFolder = () => {
        const sep = instance.instancePath.includes("/") ? "/" : "\\";
        invoke("open_directory", {
            path: `${instance.instancePath}${sep}screenshots`,
        });
    };

    return (
        <div className="flex-1 flex flex-col min-h-0 px-7 pt-5 pb-5">
            <div className="flex-shrink-0 flex gap-2.5 mb-3.5">
                <div className="flex-1"/>
                <Button
                    variant="bordered"
                    size="sm"
                    startContent={<I.folder size={13}/>}
                    onPress={handleOpenFolder}
                >
                    Open Folder
                </Button>
                <Button
                    variant="bordered"
                    size="sm"
                    startContent={<I.refresh size={13}/>}
                    onPress={fetchScreenshots}
                >
                    Refresh
                </Button>
            </div>

            {screenshots.length === 0 ? (
                <div className="flex-1 flex items-center justify-center text-ink-3 text-sm">
                    <div className="text-center">
                        <I.image size={32} className="mx-auto mb-3 opacity-40"/>
                        <div>No screenshots yet</div>
                        <div className="text-xs mt-1 text-ink-4">
                            Press F2 in-game to take a screenshot
                        </div>
                    </div>
                </div>
            ) : (
                <div className="flex-1 overflow-y-auto">
                    <div className="grid grid-cols-3 gap-3">
                        {screenshots.map((ss) => (
                            <Card
                                key={ss.fileName}
                                className="p-0 overflow-hidden border border-line group relative"
                                style={cardSurfaceStyle}
                            >
                                <div className="h-[160px] relative bg-bg-1">
                                    {imageCache[ss.fileName] ? (
                                        <img
                                            src={imageCache[ss.fileName]}
                                            alt={ss.fileName}
                                            className="w-full h-full object-cover"
                                        />
                                    ) : (
                                        <div className="w-full h-full flex items-center justify-center">
                                            <div className="w-5 h-5 border-2 border-ink-3 border-t-transparent rounded-full animate-spin"/>
                                        </div>
                                    )}
                                    <Button
                                        isIconOnly
                                        variant="flat"
                                        size="sm"
                                        aria-label="Delete"
                                        className="absolute top-2 right-2 w-7 h-7 min-w-0 bg-[rgba(0,0,0,0.6)] backdrop-blur-sm opacity-0 group-hover:opacity-100 transition-opacity"
                                        onPress={() => handleDelete(ss.fileName)}
                                    >
                                        <I.trash size={13}/>
                                    </Button>
                                </div>
                                <div className="px-3 py-2.5 text-[0.6875rem] text-ink-3 font-mono flex items-center gap-2">
                                    <I.image size={11}/>
                                    <span className="truncate flex-1">{ss.fileName}</span>
                                    <span>{formatBytes(ss.fileSizeBytes)}</span>
                                </div>
                            </Card>
                        ))}
                    </div>
                </div>
            )}
        </div>
    );
}
