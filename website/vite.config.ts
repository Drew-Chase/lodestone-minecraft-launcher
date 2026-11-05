import {defineConfig} from "vite";
import react from "@vitejs/plugin-react";
import {copyFileSync, readFileSync} from "node:fs";
import {join, resolve} from "node:path";
import {fileURLToPath} from "node:url";
import type {Plugin} from "vite";

// The Linux installer is one script, kept beside the launcher in res/ and shipped with each release. The site serves
// the same file at /install, so `curl -fsSL https://lodestonelauncher.com/install | sh` runs what the release ships.
const installer = fileURLToPath(new URL("../res/installer.sh", import.meta.url));

function serveInstaller(): Plugin {
    let outDir = "";
    return {
        name: "lodestone-installer",
        configResolved(config) {
            outDir = resolve(config.root, config.build.outDir);
        },
        configureServer(server) {
            server.middlewares.use("/install", (_req, res) => {
                res.setHeader("Content-Type", "text/plain; charset=utf-8");
                res.end(readFileSync(installer));
            });
        },
        writeBundle() {
            copyFileSync(installer, join(outDir, "install"));
        }
    };
}

export default defineConfig({
    plugins: [react(), serveInstaller()],
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
