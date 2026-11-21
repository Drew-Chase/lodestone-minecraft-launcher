// Mixes a voice-over take and a music bed onto the demo video.
// The take is cut at its paragraph pauses and each paragraph is placed at its scene's cue time.
// usage: node mix.mjs --silences <vo.mp3>       lists the take's pauses, to pick the 11 between paragraphs
//        node mix.mjs <video> <vo.mp3> <music.mp3> <out.mp4> <cuts>
// <cuts> is 11 comma-separated "silenceStart|silenceEnd" pairs, one per paragraph break, in order. A take also pauses
// inside a paragraph (after a question, between sentences), so match the pauses against the script's paragraphs rather
// than taking the 11 longest. The music bed is turned down 11 dB and ducked ~9 dB more under the voice.
import { execFileSync, spawnSync } from 'node:child_process';

if (process.argv[2] === '--silences') {
  const log = spawnSync('ffmpeg', ['-hide_banner', '-nostats', '-i', process.argv[3], '-af', 'silencedetect=noise=-45dB:d=0.35', '-f', 'null', '-']).stderr.toString();
  const starts = [...log.matchAll(/silence_start: ([\d.]+)/g)].map((m) => m[1]);
  const ends = [...log.matchAll(/silence_end: ([\d.]+) \| silence_duration: ([\d.]+)/g)];
  ends.forEach((m, i) => console.log(`${starts[i]}|${m[1]}   (${Number(m[2]).toFixed(2)}s)`));
  process.exit(0);
}
const [video, vo, music, out, cutsArg] = process.argv.slice(2);
const DUR = Number(process.env.DEMO_DURATION || 96.6); // the video's length; render.mjs prints it as "total"
// Cue start for each of the 12 paragraphs: the line starts in voiceover.srt, one paragraph per scene. Keep these in step
// with the scene starts stage.html prints (render.mjs logs "scenes").
const CUES = [0.6, 5.5, 15.6, 24.0, 32.0, 40.5, 49.5, 60.0, 69.8, 78.5, 84.3, 90.9];
const GAP = 0.35;

// cuts: 11 pairs "silenceStart|silenceEnd" between paragraphs
const pairs = cutsArg.split(',').map((p) => p.split('|').map(Number));
const voDur = Number(execFileSync('ffprobe', ['-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0', vo]).toString());
const paras = [];
for (let i = 0; i < 12; i++) {
  const a = i === 0 ? 0 : Math.max(0, pairs[i - 1][1] - 0.06);
  const b = i === 11 ? voDur : pairs[i][0] + 0.12;
  paras.push([a, b]);
}
let prevEnd = 0;
const placed = paras.map(([a, b], i) => {
  const at = Math.max(CUES[i], prevEnd + GAP);
  prevEnd = at + (b - a);
  return { a, b, at };
});
for (const [i, p] of placed.entries()) console.error(`p${i + 1} ${p.a.toFixed(2)}-${p.b.toFixed(2)} (${(p.b - p.a).toFixed(1)}s) at ${p.at.toFixed(2)} cue ${CUES[i]} ${p.at - CUES[i] > .05 ? `(+${(p.at - CUES[i]).toFixed(2)}s)` : ''}`);

// voice: each paragraph trimmed, delayed into place, summed; then levelled to -16 LUFS
const f = [];
placed.forEach((p, i) => {
  const ms = Math.round(p.at * 1000);
  f.push(`[1:a]atrim=${p.a}:${p.b},asetpts=PTS-STARTPTS,afade=t=in:d=0.02,afade=t=out:st=${(p.b - p.a - 0.04).toFixed(3)}:d=0.04,aresample=48000,aformat=channel_layouts=mono,adelay=${ms}:all=1[v${i}]`);
});
f.push(`${placed.map((_, i) => `[v${i}]`).join('')}amix=inputs=12:normalize=0,apad=whole_dur=${DUR},atrim=0:${DUR},highpass=f=80,acompressor=threshold=-20dB:ratio=3:attack=5:release=120:makeup=2,loudnorm=I=-16:TP=-2:LRA=7:linear=true,aresample=48000,pan=stereo|c0=c0|c1=c0,asplit=2[vox][key]`);
// music: brought down to a bed, ducked a further ~9 dB under the voice, faded at the end
f.push(`[2:a]aresample=48000,atrim=0:${DUR},volume=-11dB,afade=t=out:st=${DUR - 2.2}:d=2.2[bed]`);
f.push(`[bed][key]sidechaincompress=threshold=0.02:ratio=6:attack=40:release=650:makeup=1[duck]`);
f.push(`[duck][vox]amix=inputs=2:normalize=0,alimiter=limit=0.84:level=false[mix]`);

execFileSync('ffmpeg', ['-hide_banner', '-loglevel', 'error', '-y', '-i', video, '-i', vo, '-i', music,
  '-filter_complex', f.join(';'), '-map', '0:v', '-map', '[mix]', '-c:v', 'copy', '-c:a', 'aac', '-b:a', '256k', '-ar', '48000',
  '-t', String(DUR), '-movflags', '+faststart', out], { stdio: 'inherit' });
console.error('wrote', out);
