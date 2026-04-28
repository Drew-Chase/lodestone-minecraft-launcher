import {useEffect, useState} from "react";
import {invoke} from "@tauri-apps/api/core";

/**
 * Load an instance image (icon.png or banner.png) from the instance directory.
 * Returns a blob URL if the image exists, null otherwise.
 */
export function useInstanceImage(instancePath: string, imageName: "icon.png" | "banner.png"): string | null {
    const [url, setUrl] = useState<string | null>(null);
    useEffect(() => {
        let revoke: string | null = null;
        invoke<number[]>("read_instance_image", {instancePath, imageName})
            .then((bytes) => {
                const blob = new Blob([new Uint8Array(bytes)], {type: "image/png"});
                revoke = URL.createObjectURL(blob);
                setUrl(revoke);
            })
            .catch(() => setUrl(null));
        return () => {
            if (revoke) URL.revokeObjectURL(revoke);
        };
    }, [instancePath, imageName]);
    return url;
}
