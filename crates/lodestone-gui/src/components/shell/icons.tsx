import {Icon as IconifyIcon} from "@iconify-icon/react";
import React from "react";

// Unified icon helper so consumers can write <I.home size={20}/> like the design prototype.
// Maps design icon names to public Iconify icons (mostly lucide).
type IconProps = {
    size?: number | string;
    className?: string;
    style?: React.CSSProperties;
};

const make = (name: string) => {
    const Comp = ({size = 18, className, style}: IconProps) => (
        <IconifyIcon
            icon={name}
            width={size}
            height={size}
            className={className}
            style={style}
        />
    );
    Comp.displayName = `Icon(${name})`;
    return Comp;
};

export const I = {
    play: make("mdi:play"),
    pause: make("mdi:pause"),
    home: make("lucide:home"),
    compass: make("lucide:compass"),
    box: make("lucide:package"),
    grid: make("lucide:layout-grid"),
    list: make("lucide:list"),
    table: make("lucide:table"),
    server: make("lucide:server"),
    settings: make("lucide:settings"),
    search: make("lucide:search"),
    download: make("lucide:download"),
    plus: make("lucide:plus"),
    filter: make("lucide:filter"),
    bell: make("lucide:bell"),
    chevRight: make("lucide:chevron-right"),
    chevDown: make("lucide:chevron-down"),
    user: make("lucide:user"),
    clock: make("lucide:clock"),
    globe: make("lucide:globe"),
    x: make("lucide:x"),
    check: make("lucide:check"),
    star: make("lucide:star"),
    trash: make("lucide:trash-2"),
    folder: make("lucide:folder"),
    cpu: make("lucide:cpu"),
    zap: make("lucide:zap"),
    users: make("lucide:users"),
    lock: make("lucide:lock"),
    refresh: make("lucide:refresh-cw"),
    more: make("lucide:more-horizontal"),
    external: make("lucide:external-link"),
    heart: make("lucide:heart"),
    terminal: make("lucide:terminal"),
    image: make("lucide:image"),
    tag: make("lucide:tag"),
    shield: make("lucide:shield"),
    pickaxe: make("mdi:pickaxe"),
    sword: make("lucide:sword"),
    pin: make("lucide:pin"),
    hardDrive: make("lucide:hard-drive"),
    copy: make("lucide:copy"),
    upload: make("lucide:upload"),
    signal: make("lucide:signal"),
    wifi: make("lucide:wifi"),
    headset: make("lucide:headphones"),
    gamepad: make("lucide:gamepad-2"),
    message: make("lucide:message-square"),
    pkg: make("lucide:package-2"),
    minimize: make("material-symbols:minimize-rounded"),
    maximize: make("material-symbols:square-outline-rounded"),
    close: make("material-symbols:close-rounded"),
};
