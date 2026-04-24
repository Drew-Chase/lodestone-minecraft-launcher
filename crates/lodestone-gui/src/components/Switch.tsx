import {extendVariants, Switch as HeroSwitch} from "@heroui/react";

// Drop-in override for HeroUI's Switch that renders the thumb with the track's
// foreground color when the switch is selected, instead of HeroUI's hardcoded
// `bg-white` thumb. Matches the design's Minecraft-style toggle (dark handle on
// a bright track).
//
// HeroUI's Switch theme (see @heroui/theme .../components/toggle.js) only
// applies color variants to the `wrapper` slot, so the thumb stays white across
// every color. HeroUI also doesn't expose a thumb token in `heroui()`'s theme
// config, so the idiomatic path is to `extendVariants` at the component layer.
// The base slot already carries `group`, so `group-data-[selected=true]:` on
// the thumb correctly reacts to the base element's `data-selected` attribute.
export const Switch = extendVariants(HeroSwitch, {
    variants: {
        color: {
            default: {thumb: "group-data-[selected=true]:bg-default-foreground"},
            primary: {thumb: "group-data-[selected=true]:bg-primary-foreground"},
            secondary: {thumb: "group-data-[selected=true]:bg-secondary-foreground"},
            success: {thumb: "group-data-[selected=true]:bg-success-foreground"},
            warning: {thumb: "group-data-[selected=true]:bg-warning-foreground"},
            danger: {thumb: "group-data-[selected=true]:bg-danger-foreground"},
        },
    },
});
