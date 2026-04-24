import React from "react";
import {Modal, ModalBody, ModalContent} from "@heroui/react";
import {I} from "../shell/icons";

type ModalShellProps = {
    isOpen: boolean;
    onClose: () => void;
    title: string;
    subtitle?: string;
    icon?: (p: {size?: number}) => React.ReactElement;
    // CSS color string (or custom-property reference) used for the accent glow,
    // border tint, icon color, and top accent line.
    accent?: string;
    // HeroUI Modal size token — maps to a max-width. Defaults to "2xl" (~672px)
    // which is close to the design's 720px default.
    size?: "sm" | "md" | "lg" | "xl" | "2xl" | "3xl" | "4xl" | "5xl";
    children?: React.ReactNode;
    footer?: React.ReactNode;
};

// Shared shell for every quick-action modal. Wraps HeroUI's Modal (for focus trap,
// ESC-to-close, and animated backdrop) with the design's accent-glow header, icon
// square, top accent line, and optional footer.
export default function ModalShell({
                                       isOpen,
                                       onClose,
                                       title,
                                       subtitle,
                                       icon,
                                       accent = "var(--mc-green)",
                                       size = "2xl",
                                       children,
                                       footer,
                                   }: ModalShellProps) {
    const IconC = icon;

    return (
        <Modal
            isOpen={isOpen}
            onClose={onClose}
            backdrop="blur"
            hideCloseButton
            size={size}
            placement="center"
            scrollBehavior="inside"
            classNames={{
                backdrop: "!bg-[rgba(6,10,8,0.62)] backdrop-blur-md backdrop-saturate-[1.1]",
                base: "!rounded-[18px] overflow-hidden",
                body: "p-0",
            }}
        >
            <ModalContent
                className="border"
                style={{
                    // Layered gradient + accent border + glow shadow, all dependent on
                    // the dynamic accent color — keep inline.
                    background:
                        "linear-gradient(180deg, rgba(28,34,30,0.96), rgba(18,22,20,0.96))",
                    borderColor: `color-mix(in oklab, ${accent} 24%, var(--line))`,
                    boxShadow: `0 30px 80px rgba(0,0,0,0.6), inset 0 0 0 1px rgba(255,255,255,0.03), 0 0 60px color-mix(in oklab, ${accent} 18%, transparent)`,
                }}
            >
                {() => (
                    <>
                        {/* Header */}
                        <div className="relative flex items-center gap-3.5 px-[22px] pt-[18px] pb-4 border-b border-line">
                            <div
                                className="flex-shrink-0 w-10 h-10 rounded-[10px] flex items-center justify-center border"
                                style={{
                                    background: `color-mix(in oklab, ${accent} 18%, transparent)`,
                                    borderColor: `color-mix(in oklab, ${accent} 35%, transparent)`,
                                    color: accent,
                                }}
                            >
                                {IconC && <IconC size={20}/>}
                            </div>
                            <div className="flex-1 min-w-0">
                                <div className="text-base font-bold tracking-tight">{title}</div>
                                {subtitle && (
                                    <div className="text-xs text-ink-3 mt-0.5">{subtitle}</div>
                                )}
                            </div>
                            <button
                                type="button"
                                onClick={onClose}
                                aria-label="Close"
                                className="w-8 h-8 rounded-lg bg-transparent border border-line text-ink-2 cursor-pointer flex items-center justify-center hover:bg-[rgba(255,255,255,0.04)]"
                            >
                                <I.x size={14}/>
                            </button>
                            {/* Top accent gradient line */}
                            <div
                                className="absolute top-0 left-[10%] right-[10%] h-px opacity-60"
                                style={{
                                    background: `linear-gradient(90deg, transparent, ${accent}, transparent)`,
                                }}
                            />
                        </div>

                        {/* Body */}
                        <ModalBody className="!p-0">
                            <div className="px-[22px] py-5 overflow-auto">{children}</div>
                        </ModalBody>

                        {/* Footer */}
                        {footer && (
                            <div className="px-[22px] py-3.5 border-t border-line flex justify-end gap-2.5 bg-[rgba(0,0,0,0.25)]">
                                {footer}
                            </div>
                        )}
                    </>
                )}
            </ModalContent>
        </Modal>
    );
}
