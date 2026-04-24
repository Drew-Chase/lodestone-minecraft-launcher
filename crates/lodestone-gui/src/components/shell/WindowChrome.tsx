import {Button, ButtonGroup} from "@heroui/react";
import {getCurrentWindow} from "@tauri-apps/api/window";
import {I} from "./icons";

// Platform detection at module load. Read from `navigator.userAgent`
// because it's synchronous inside both WKWebView (macOS) and WebKitGTK
// (Linux) without requiring the async `@tauri-apps/plugin-os` call.
//
// - macOS: `titleBarStyle: "Overlay"` (see tauri.macos.conf.json) draws
//   native traffic lights overlaid on our content. We hide the HTML
//   min/max/close and reserve ~80px on the left for them.
// - Linux: `decorations: true` (see tauri.linux.conf.json) asks the WM
//   to draw a native titlebar strip ABOVE the webview. That strip
//   already contains native min/max/close, so we hide our HTML buttons
//   to avoid duplicating them. No left inset needed — the native strip
//   is separate from the webview, not overlaid on it.
// - Windows: no native chrome (decorations: false in base config), so
//   the HTML buttons remain the only window controls.
const IS_MACOS: boolean = typeof navigator !== "undefined"
    && /macintosh|mac os x/i.test(navigator.userAgent);
const IS_LINUX: boolean = typeof navigator !== "undefined"
    && !IS_MACOS
    && /linux/i.test(navigator.userAgent);
const HAS_NATIVE_CHROME: boolean = IS_MACOS || IS_LINUX;

// App-level titlebar: drag region, version label, functional Tauri window controls.
export default function WindowChrome()
{
    const appWindow = getCurrentWindow();
    return (
        <div
            className={
                "flex flex-row items-center sticky top-0 w-full select-none h-8 z-[51] bg-transparent"
                // Inset left on macOS so the native traffic lights
                // (~70px wide at default density) don't collide with
                // our titlebar content. The left spacer below still
                // renders on non-Mac platforms for symmetry.
                + (IS_MACOS ? " pl-[80px]" : "")
            }
            data-tauri-drag-region=""
        >
            {/* Left: brand spacer (keeps layout symmetric; leave blank for drag region).
                Hidden on macOS — the pl-[80px] on the root covers the
                traffic-light gap. Still rendered on Linux: the native GTK
                titlebar is a separate strip above the webview, so our
                layout here doesn't need a horizontal shift. */}
            {!IS_MACOS && (
                <div className="w-[120px]" data-tauri-drag-region=""/>
            )}

            {/* Center: version label */}
            <div
                className="flex-1 flex justify-center text-ink-3 font-mono select-none text-[0.6875rem] tracking-[0.3px]"
                data-tauri-drag-region=""
            >
                LODESTONE · v2.1.4
            </div>

            {/* Right: window controls. Hidden on both macOS (native
                traffic lights via titleBarStyle: "Overlay") and Linux
                (native WM titlebar drawn above the webview via
                decorations: true). Shown only on Windows where the base
                config keeps decorations: false. */}
            {!HAS_NATIVE_CHROME && (
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
            )}
        </div>
    );
}
