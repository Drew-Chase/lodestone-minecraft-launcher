import {defineConfig} from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
    plugins: [react()],
    clearScreen: false,
    server: {
        host: true,
        port: 3000,
        strictPort: true,
        hmr: {
            protocol: "ws",
            host: "localhost",
            port: 3000,
            clientPort: 3000,
            overlay: true
        },
        // The PHP API runs on its own port in development (see the dev:api script).
        proxy: {
            "/api": {
                target: "http://localhost:8000",
                changeOrigin: true,
                secure: false
            }
        }
    },
    build: {
        outDir: "dist/"
    }
});
