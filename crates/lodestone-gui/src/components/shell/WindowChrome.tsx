import {Button, ButtonGroup} from "@heroui/react";
import {getCurrentWindow} from "@tauri-apps/api/window";
import {I} from "./icons";

// App-level titlebar: drag region, version label, functional Tauri window controls.
export default function WindowChrome()
{
    const appWindow = getCurrentWindow();
    return (
        <div
            className="flex flex-row items-center sticky top-0 w-full select-none h-8 z-[51] bg-transparent"
            data-tauri-drag-region=""
        >
            {/* Left: brand spacer (keeps layout symmetric; leave blank for drag region) */}
            <div className="w-[120px]" data-tauri-drag-region=""/>

            {/* Center: version label */}
            <div
                className="flex-1 flex justify-center text-ink-3 font-mono select-none text-[0.6875rem] tracking-[0.3px]"
                data-tauri-drag-region=""
            >
                LODESTONE · v2.1.4
            </div>

            {/* Right: window controls */}
            <div className="flex flex-row ml-auto">
                <ButtonGroup className="h-8">
                    <Button
                        variant="light"
                        radius="none"
                        className="min-w-0 h-8 text-ink-2"
                        onPress={() => appWindow.minimize()}
                    >
                        <I.minimize size={18}/>
                    </Button>
                    <Button
                        variant="light"
                        radius="none"
                        className="min-w-0 h-8 text-ink-2"
                        onPress={() => appWindow.toggleMaximize()}
                    >
                        <I.maximize size={14}/>
                    </Button>
                    <Button
                        variant="light"
                        color="danger"
                        radius="none"
                        className="min-w-0 h-8"
                        onPress={() => appWindow.close()}
                    >
                        <I.close size={18}/>
                    </Button>
                </ButtonGroup>
            </div>
        </div>
    );
}
