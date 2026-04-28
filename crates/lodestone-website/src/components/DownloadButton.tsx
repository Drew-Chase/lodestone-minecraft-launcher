import {Ic} from "./Icons.tsx";
import {useReleaseContext, detectOS, getOSLabel} from "../hooks/useRelease.ts";

export default function DownloadButton({className, size}: { className?: string; size?: number }) {
    const {available, loading, downloadUrl} = useReleaseContext();
    const os = detectOS();
    const osLabel = getOSLabel(os);

    if (loading) {
        return (
            <span className={className ?? "btn btn-primary btn-lg"} style={{opacity: 0.6, pointerEvents: "none"}}>
                <Ic.download size={size ?? 15}/> Loading...
            </span>
        );
    }

    if (!available) {
        return (
            <span className={className ?? "btn btn-primary btn-lg"} style={{opacity: 0.7, cursor: "default"}}>
                <Ic.download size={size ?? 15}/> Coming Soon
            </span>
        );
    }

    return (
        <a className={className ?? "btn btn-primary btn-lg"} href={downloadUrl ?? "#download"}>
            <Ic.download size={size ?? 15}/> Download for {osLabel}
        </a>
    );
}
