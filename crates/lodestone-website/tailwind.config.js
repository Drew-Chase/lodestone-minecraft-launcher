/** @type {import('tailwindcss').Config} */
export default {
    content: [
        "./index.html",
        "./src/**/*.{js,ts,jsx,tsx}",
    ],
    theme: {
        fontFamily: {
            sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'system-ui', 'sans-serif'],
            display: ['Space Grotesk', 'Inter', 'sans-serif'],
            mono: ['JetBrains Mono', 'ui-monospace', 'SF Mono', 'Menlo', 'monospace'],
        },
        extend: {
            colors: {
                'mc-green': '#22ff84',
                'mc-green-dim': '#18c968',
                'mc-green-deep': '#0a5c33',
                'violet': '#9747ff',
                'amber': '#ffb545',
                'cyan': '#47d9ff',
                'pink': '#ff5ec8',
                'red': '#ff5a7a',
                'bg-0': '#08090a',
                'bg-1': '#0e1012',
                'bg-2': '#15181c',
                'bg-3': '#1d2127',
                'bg-4': '#272c34',
            },
        },
    },
    darkMode: "class",
    plugins: []
}
