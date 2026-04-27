import {useCallback, useEffect, useState} from "react";
import {Button} from "@heroui/react";
import {invoke} from "@tauri-apps/api/core";
import {I} from "../shell/icons";
import type {Instance} from "../library/instances";
import GalleryTab from "../discover/GalleryTab";

type Props = {
    instance: Instance;
};

interface ScreenshotEntry {
    fileName: string;
    fileSizeBytes: number;
    lastModified: string;
}

export default function ScreenshotsTab({instance}: Props) {
    const [screenshots, setScreenshots] = useState<ScreenshotEntry[]>([]);
    const [imageUrls, setImageUrls] = useState<string[]>([]);

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

    // Load all screenshot images as blob URLs for the gallery
    useEffect(() => {
        let cancelled = false;
        const urls: string[] = [];

        (async () => {
            for (const ss of screenshots) {
                if (cancelled) break;
                try {
                    const bytes = await invoke<number[]>("read_instance_screenshot", {
                        instancePath: instance.instancePath,
                        fileName: ss.fileName,
                    });
                    const blob = new Blob([new Uint8Array(bytes)], {type: "image/png"});
                    urls.push(URL.createObjectURL(blob));
                } catch {
                    // skip failed loads
                }
            }
            if (!cancelled) setImageUrls([...urls]);
        })();

        return () => {
            cancelled = true;
            for (const url of urls) URL.revokeObjectURL(url);
        };
    }, [screenshots, instance.instancePath]);

    const handleOpenFolder = () => {
        const sep = instance.instancePath.includes("/") ? "/" : "\\";
        invoke("open_directory", {path: `${instance.instancePath}${sep}screenshots`});
    };

    return (
        <div className="flex-1 flex flex-col min-h-0 px-7 pt-5 pb-5">
            <div className="flex-shrink-0 flex gap-2.5 mb-3.5">
                <div className="text-xs text-ink-3 font-mono self-center">
                    {screenshots.length} screenshot{screenshots.length !== 1 ? "s" : ""}
                </div>
                <div className="flex-1"/>
                <Button variant="bordered" size="sm" startContent={<I.folder size={13}/>} onPress={handleOpenFolder}>
                    Open Folder
                </Button>
                <Button variant="bordered" size="sm" startContent={<I.refresh size={13}/>} onPress={fetchScreenshots}>
                    Refresh
                </Button>
            </div>

            <div className="flex-1 overflow-y-auto">
                {screenshots.length === 0 ? (
                    <div className="flex-1 flex items-center justify-center text-ink-3 text-sm h-full">
                        <div className="text-center">
                            <I.image size={32} className="mx-auto mb-3 opacity-40"/>
                            <div>No screenshots yet</div>
                            <div className="text-xs mt-1 text-ink-4">Press F2 in-game to take a screenshot</div>
                        </div>
                    </div>
                ) : (
                    <GalleryTab images={imageUrls} title={instance.name}/>
                )}
            </div>
        </div>
    );
}
