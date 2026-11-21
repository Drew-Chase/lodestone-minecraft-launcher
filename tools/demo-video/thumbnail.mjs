// Renders thumbnail.html to a 1920x1080 PNG (YouTube's 16:9, under its 2 MB limit after the JPEG copy).
// usage: node thumbnail.mjs <out.png>
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { TOOLS, prepareStage, findChrome } from './env.mjs';

const out = path.resolve(process.argv[2] || 'thumbnail.png');
const stage = prepareStage();
fs.copyFileSync(path.join(TOOLS, 'thumbnail.html'), path.join(stage, 'thumbnail.html'));
const r = spawnSync(findChrome(), ['--allow-file-access-from-files', '--hide-scrollbars', '--force-device-scale-factor=1.5',
  '--window-size=1280,720', '--virtual-time-budget=4000', `--screenshot=${out}`, 'file://' + path.join(stage, 'thumbnail.html')], { stdio: 'inherit' });
process.exit(r.status ?? 1);
