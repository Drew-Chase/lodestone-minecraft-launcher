// Shared paths for the demo-video tools. Everything generated lives under target/demo-video (or $DEMO_WORK).
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

export const TOOLS = path.dirname(new URL(import.meta.url).pathname);
export const REPO = path.resolve(TOOLS, '../..');
export const WORK = process.env.DEMO_WORK || path.join(REPO, 'target/demo-video');

// Copies stage.html and the platform logos next to the extracted frames, and returns that folder.
export function prepareStage() {
  const stage = path.join(WORK, 'stage');
  fs.mkdirSync(path.join(stage, 'plat'), { recursive: true });
  fs.copyFileSync(path.join(TOOLS, 'stage.html'), path.join(stage, 'stage.html'));
  for (const f of fs.readdirSync(path.join(TOOLS, 'plat'))) fs.copyFileSync(path.join(TOOLS, 'plat', f), path.join(stage, 'plat', f));
  if (!fs.existsSync(path.join(stage, 'frames/manifest.js'))) throw new Error(`no frames in ${stage}/frames: run tools/demo-video/record.sh extract first`);
  return stage;
}

// Chrome's headless shell: $CHROME_HEADLESS_SHELL, else the newest one puppeteer cached
// (`npx @puppeteer/browsers install chrome-headless-shell@stable` puts one there).
export function findChrome() {
  if (process.env.CHROME_HEADLESS_SHELL) return process.env.CHROME_HEADLESS_SHELL;
  const root = path.join(os.homedir(), '.cache/puppeteer/chrome-headless-shell');
  const versions = fs.existsSync(root) ? fs.readdirSync(root).sort().reverse() : [];
  for (const v of versions) {
    const bin = path.join(root, v, 'chrome-headless-shell-linux64/chrome-headless-shell');
    if (fs.existsSync(bin)) return bin;
  }
  throw new Error('chrome-headless-shell not found: set CHROME_HEADLESS_SHELL or run `npx @puppeteer/browsers install chrome-headless-shell@stable`');
}
