import {Button, ButtonGroup} from "@heroui/react";
import {getCurrentWindow} from "@tauri-apps/api/window";
import {I} from "./icons";

// App-level titlebar: drag region, version label, functional Tauri window controls.
export default function WindowChrome() {
    const appWindow = getCurrentWindow();
    return (
        <div
            className="flex flex-row items-center sticky top-0 w-full select-none border-b border-line"
            style={{
                height: "2rem",
                zIndex: 51,
                background: "rgba(8,9,10,0.85)",
                backdropFilter: "blur(12px) saturate(150%)",
            }}
            data-tauri-drag-region=""
        >
            {/* Left: brand spacer (keeps layout symmetric; leave blank for drag region) */}
            <div style={{width: 120}} data-tauri-drag-region=""/>

            {/* Center: version label */}
            <div
                className="flex-1 flex justify-center text-ink-3 font-mono select-none"
                style={{fontSize: 11, letterSpacing: 0.3}}
                data-tauri-drag-region=""
            >
                LODESTONE · v2.1.4
            </div>

            {/* Right: window controls */}
            <div className="flex flex-row ml-auto">
                <ButtonGroup style={{height: "2rem"}}>
                    <Button
                        variant="light"
                        className="min-w-0 h-[2rem] text-ink-2"
                        radius="none"
                        onPress={() => appWindow.minimize()}
                    >
                        <I.minimize size={18}/>
                    </Button>
                    <Button
                        variant="light"
                        className="min-w-0 h-[2rem] text-ink-2"
                        radius="none"
                        onPress={() => appWindow.toggleMaximize()}
                    >
                        <I.maximize size={14}/>
                    </Button>
                    <Button
                        variant="light"
                        color="danger"
                        className="min-w-0 h-[2rem]"
                        radius="none"
                        onPress={() => appWindow.close()}
                    >
                        <I.close size={18}/>
                    </Button>
                </ButtonGroup>
            </div>
        </div>
    );
}
