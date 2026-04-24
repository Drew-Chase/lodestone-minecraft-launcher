import {type ReactNode} from "react";
import {Card} from "@heroui/react";
import {I} from "../shell/icons";
import {Switch} from "../Switch";
import {cardSurfaceStyle} from "../surfaces";

// Card wrapper used around groups of setting rows. Optional title + description
// stack above the row group.
export function SettingCard({
                                title,
                                desc,
                                children,
                                className = "",
                            }: {
    title?: string;
    desc?: string;
    children: ReactNode;
    className?: string;
}) {
    return (
        <Card
            className={`p-[22px] mb-4 border border-line ${className}`}
            style={cardSurfaceStyle}
        >
            {title && (
                <div className="text-sm font-semibold mb-1">{title}</div>
            )}
            {desc && (
                <div className="text-xs text-ink-3 mb-4">{desc}</div>
            )}
            {children}
        </Card>
    );
}

// Row with a label/description pair on the left and a toggle on the right.
// Uses our overridden Switch so the thumb renders dark when on.
export function ToggleRow({
                              label,
                              desc,
                              on,
                              last,
                          }: {
    label: string;
    desc?: string;
    on?: boolean;
    last?: boolean;
}) {
    return (
        <div
            className={`flex items-center gap-3.5 px-[18px] py-4 ${
                last ? "" : "border-b border-line"
            }`}
        >
            <div className="flex-1">
                <div className="text-[0.8125rem] font-semibold">{label}</div>
                {desc && (
                    <div className="text-[0.6875rem] text-ink-3 mt-0.5">{desc}</div>
                )}
            </div>
            <Switch
                defaultSelected={on}
                size="sm"
                color="success"
                aria-label={label}
            />
        </div>
    );
}

// Row with a label on the left and a (visual) select button on the right.
// Renamed from the modals' SelectRow to avoid collision at import sites.
export function SelectSettingRow({
                                     label,
                                     value,
                                     last,
                                 }: {
    label: string;
    value: string;
    last?: boolean;
}) {
    return (
        <div
            className={`flex items-center gap-3.5 px-[18px] py-3.5 ${
                last ? "" : "border-b border-line"
            }`}
        >
            <div className="flex-1 text-[0.8125rem] font-medium">{label}</div>
            <button
                type="button"
                className="inline-flex items-center gap-2 px-3 py-2 rounded-md bg-[rgba(255,255,255,0.04)] border border-line text-ink-1 text-[0.8125rem] cursor-pointer hover:bg-[rgba(255,255,255,0.08)] transition-colors"
            >
                {value}
                <I.chevDown size={11}/>
            </button>
        </div>
    );
}
