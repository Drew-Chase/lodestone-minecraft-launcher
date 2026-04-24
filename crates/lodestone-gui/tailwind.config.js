import {heroui} from "@heroui/react";

/** @type {import('tailwindcss').Config} */
export default {
    content: [
        "./index.html",
        "./src/**/*.{js,ts,jsx,tsx}",
        "./node_modules/@heroui/theme/dist/**/*.{js,ts,jsx,tsx}"
    ],
    theme: {
        extend: {
            colors: {
                "mc-green": {
                    DEFAULT: "#22ff84",
                    dim: "#18c968",
                    deep: "#0a5c33",
                },
                "accent-violet": "#9747ff",
                "accent-amber": "#ffb545",
                "accent-cyan": "#47d9ff",
                "accent-pink": "#ff5ec8",
                "bg-0": "#08090a",
                "bg-1": "#0e1012",
                "bg-2": "#15181c",
                "bg-3": "#1d2127",
                "bg-4": "#272c34",
                "ink-0": "#ffffff",
                "ink-1": "rgba(255,255,255,0.92)",
                "ink-2": "rgba(255,255,255,0.68)",
                "ink-3": "rgba(255,255,255,0.45)",
                "ink-4": "rgba(255,255,255,0.28)",
                line: "rgba(255,255,255,0.07)",
                "line-strong": "rgba(255,255,255,0.12)",
            },
            fontFamily: {
                sans: ["Inter", "-apple-system", "BlinkMacSystemFont", "system-ui", "sans-serif"],
                mono: ["JetBrains Mono", "ui-monospace", "SF Mono", "Menlo", "monospace"],
            },
            borderRadius: {
                sm: "8px",
                md: "12px",
                lg: "18px",
                xl: "24px",
            },
            boxShadow: {
                "mc-glow": "0 8px 24px -6px rgba(34,255,132,0.35)",
                "mc-glow-strong": "0 12px 32px -6px rgba(34,255,132,0.5)",
            },
        },
    },
    darkMode: "class",
    plugins: [heroui({
        themes: {
            light: {
                colors: {
                    primary: {
                        DEFAULT: "#22ff84",
                        foreground: "#062814",
                    },
                    secondary: "#2b2b2b",
                    background: "#e3e3ea",
                }
            },
            dark: {
                colors: {
                    primary: {
                        DEFAULT: "#22ff84",
                        foreground: "#062814",
                    },
                    secondary: "#eaeaea",
                    background: "#08090a",
                    content1: "#0e1012",
                    content2: "#15181c",
                    content3: "#1d2127",
                    content4: "#272c34",
                    success: {
                        DEFAULT: "#22ff84",
                        foreground: "#062814",
                    },
                }
            },
        }
    })]
}
