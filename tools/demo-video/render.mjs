// Renders stage.html frame by frame through the Chrome DevTools Protocol and pipes JPEGs into ffmpeg.
// usage: node render.mjs <out.mp4> [fps] [from] [to] [stillsDir]
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

import { prepareStage, findChrome } from './env.mjs';
const dir = prepareStage();
const out = process.argv[2];
const fps = Number(process.argv[3] || 60);
const from = Number(process.argv[4] || 0);
const toArg = process.argv[5];
const stills = process.argv[6];
const CHROME = findChrome();
const port = 9333;

const chrome = spawn(CHROME, [`--remote-debugging-port=${port}`, '--allow-file-access-from-files', '--window-size=1920,1080',
  '--hide-scrollbars', '--enable-gpu-rasterization', '--ignore-gpu-blocklist', `--user-data-dir=${dir}/.chrome`, 'about:blank'], { stdio: 'ignore' });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let targets;
for (let i = 0; i < 50; i++) { try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); break; } catch { await sleep(200); } }
const page = targets.find((t) => t.type === 'page');
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => ws.addEventListener('open', r));
let id = 0; const pending = new Map();
ws.addEventListener('message', (e) => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); } });
const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expr) => { const r = await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true }); if (r.result?.exceptionDetails) throw new Error(JSON.stringify(r.result.exceptionDetails)); return r.result?.result?.value; };

await send('Emulation.setDeviceMetricsOverride', { width: 1920, height: 1080, deviceScaleFactor: Number(process.env.SCALE || 1), mobile: false });
await send('Page.enable');
await send('Page.navigate', { url: 'file://' + path.join(dir, 'stage.html') });
await sleep(1500);
const total = await evaluate('window.ready'); console.error('scenes', JSON.stringify(await evaluate('window.SCENES')));
const to = toArg ? Number(toArg) : total;
console.error('total', total.toFixed(2), 's; rendering', from, '->', to.toFixed(2));

let ff;
if (!stills) {
  ff = spawn('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-y', '-f', 'image2pipe', '-framerate', String(fps), '-c:v', 'mjpeg', '-i', '-',
    '-c:v', 'libx264', '-preset', process.env.SCALE > 1 ? 'medium' : 'slow', '-crf', '16', '-pix_fmt', 'yuv420p', '-movflags', '+faststart', out], { stdio: ['pipe', 'inherit', 'inherit'] });
}
const n = Math.round((to - from) * fps);
const t0 = Date.now();
for (let i = 0; i < n; i++) {
  const t = from + i / fps;
  await evaluate(`seek(${t})`);
  const shot = await send('Page.captureScreenshot', { format: 'jpeg', quality: 95, optimizeForSpeed: true });
  const buf = Buffer.from(shot.result.data, 'base64');
  if (stills) fs.writeFileSync(path.join(stills, `t${t.toFixed(2)}.jpg`), buf);
  else if (!ff.stdin.write(buf)) await new Promise((r) => ff.stdin.once('drain', r));
  if (i % 300 === 0) console.error(`${i}/${n} ${((Date.now() - t0) / 1000).toFixed(0)}s`);
}
if (ff) { ff.stdin.end(); await new Promise((r) => ff.on('close', r)); }
ws.close(); chrome.kill();
console.error('done', ((Date.now() - t0) / 1000).toFixed(0), 's');
