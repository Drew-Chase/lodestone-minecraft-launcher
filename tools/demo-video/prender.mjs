// Parallel renderer: splits the timeline into slices, renders each slice in its own headless Chrome,
// encodes each slice with NVENC, then joins the slices without re-encoding.
// usage: node prender.mjs <out.mp4> [--workers 8] [--scale 2] [--fps 60] [--gpu] [--from s] [--to s]
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

import { prepareStage, findChrome } from './env.mjs';
const dir = prepareStage();
const CHROME = findChrome();
const args = process.argv.slice(2);
const opt = (k, d) => { const i = args.indexOf(k); return i < 0 ? d : args[i + 1]; };
const out = args[0];
const workers = Number(opt('--workers', 8));
const scale = Number(opt('--scale', 2));
const fps = Number(opt('--fps', 60));
const gpu = args.includes('--gpu');
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function openPage(port, tag) {
  const flags = [`--remote-debugging-port=${port}`, '--allow-file-access-from-files', '--hide-scrollbars', `--user-data-dir=${dir}/.chrome-${tag}`];
  if (gpu) flags.push('--use-angle=vulkan', '--enable-features=Vulkan,VulkanFromANGLE,DefaultANGLEVulkan', '--enable-gpu-rasterization', '--enable-zero-copy', '--ignore-gpu-blocklist');
  const chrome = spawn(CHROME, [...flags, 'about:blank'], { stdio: 'ignore' });
  let targets;
  for (let i = 0; i < 100; i++) { try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json(); break; } catch { await sleep(150); } }
  const ws = new WebSocket(targets.find((t) => t.type === 'page').webSocketDebuggerUrl);
  await new Promise((r) => ws.addEventListener('open', r));
  let id = 0; const pending = new Map();
  ws.addEventListener('message', (e) => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); } });
  const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
  const evaluate = async (expr) => { const r = await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true }); if (r.result?.exceptionDetails) throw new Error(JSON.stringify(r.result.exceptionDetails)); return r.result?.result?.value; };
  await send('Emulation.setDeviceMetricsOverride', { width: 1920, height: 1080, deviceScaleFactor: scale, mobile: false });
  await send('Page.enable');
  await send('Page.navigate', { url: 'file://' + path.join(dir, 'stage.html') });
  await sleep(1200);
  const total = await evaluate('window.ready');
  return { chrome, ws, send, evaluate, total };
}

function encoder(file) {
  return spawn('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-y', '-f', 'image2pipe', '-framerate', String(fps), '-c:v', 'mjpeg', '-i', '-',
    '-c:v', 'h264_nvenc', '-preset', 'p7', '-tune', 'hq', '-rc', 'vbr', '-cq', '17', '-b:v', '0', '-maxrate', '140M', '-bufsize', '280M',
    '-profile:v', 'high', '-pix_fmt', 'yuv420p', '-g', String(fps * 2), '-bf', '3', file], { stdio: ['pipe', 'inherit', 'inherit'] });
}

async function renderSlice(w, f0, f1, file, progress) {
  const p = await openPage(9400 + w, w);
  const ff = encoder(file);
  for (let f = f0; f < f1; f++) {
    await p.evaluate(`seek(${f / fps})`);
    const shot = await p.send('Page.captureScreenshot', { format: 'jpeg', quality: 95, optimizeForSpeed: true });
    const buf = Buffer.from(shot.result.data, 'base64');
    if (!ff.stdin.write(buf)) await new Promise((r) => ff.stdin.once('drain', r));
    progress[w]++;
  }
  ff.stdin.end(); await new Promise((r) => ff.on('close', r));
  p.ws.close(); p.chrome.kill();
}

const probe = await openPage(9399, 'probe');
const total = probe.total; probe.ws.close(); probe.chrome.kill();
const from = Number(opt('--from', 0)), to = Number(opt('--to', total));
const F0 = Math.round(from * fps), F1 = Math.round(to * fps), n = F1 - F0;
console.error(`frames ${n} at ${1920 * scale}x${1080 * scale}, ${workers} workers, gpu=${gpu}`);
const segDir = path.join(dir, 'segs'); fs.mkdirSync(segDir, { recursive: true });
const progress = new Array(workers).fill(0);
const t0 = Date.now();
const timer = setInterval(() => { const d = progress.reduce((a, b) => a + b, 0); const s = (Date.now() - t0) / 1000; console.error(`${d}/${n} frames, ${(d / s).toFixed(1)} fps, ${s.toFixed(0)}s`); }, 15000);
const files = [];
await Promise.all(Array.from({ length: workers }, (_, w) => {
  const a = F0 + Math.floor(n * w / workers), b = F0 + Math.floor(n * (w + 1) / workers);
  const file = path.join(segDir, `seg${String(w).padStart(2, '0')}.mp4`); files.push(file);
  return renderSlice(w, a, b, file, progress);
}));
clearInterval(timer);
files.sort();
fs.writeFileSync(path.join(segDir, 'list.txt'), files.map((f) => `file '${f}'`).join('\n'));
await new Promise((r) => spawn('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-y', '-f', 'concat', '-safe', '0', '-i', path.join(segDir, 'list.txt'), '-c', 'copy', '-movflags', '+faststart', out], { stdio: 'inherit' }).on('close', r));
const s = (Date.now() - t0) / 1000;
console.error(`done ${n} frames in ${s.toFixed(0)}s (${(n / s).toFixed(1)} fps) -> ${out}`);
