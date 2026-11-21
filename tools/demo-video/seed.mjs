// Seeds the scratch profile with demo instances, worlds, mods and servers.
import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { execFileSync } from 'node:child_process';

const PROF = process.argv[2];
const REPO = process.argv[3];
const SQLITE = process.env.SQLITE || 'sqlite3'; // sqlite3 CLI; set $SQLITE when it is not on PATH
const DB = path.join(PROF, 'config/lodestone/launcher.db');
const INST = path.join(PROF, 'data/lodestone/instances');
const now = Math.floor(Date.now() / 1000);
const H = 3600;

// ---------- tiny NBT writer ----------
const T = { byte: 1, int: 3, long: 4, string: 8, compound: 10 };
function nbtName(n) { const b = Buffer.from(n, 'utf8'); const l = Buffer.alloc(2); l.writeUInt16BE(b.length); return Buffer.concat([l, b]); }
function payload(type, v) {
  switch (type) {
    case 'byte': { const b = Buffer.alloc(1); b.writeInt8(v); return b; }
    case 'int': { const b = Buffer.alloc(4); b.writeInt32BE(v); return b; }
    case 'long': { const b = Buffer.alloc(8); b.writeBigInt64BE(BigInt(v)); return b; }
    case 'string': return nbtName(v);
    case 'compound': {
      const parts = [];
      for (const [k, [t, val]] of Object.entries(v)) parts.push(Buffer.from([T[t]]), nbtName(k), payload(t, val));
      parts.push(Buffer.from([0]));
      return Buffer.concat(parts);
    }
  }
}
const nbtFile = (root) => zlib.gzipSync(Buffer.concat([Buffer.from([10]), nbtName(''), payload('compound', root)]));

function sql(s) { execFileSync(SQLITE, [DB, s]); }
const q = (s) => s == null ? 'NULL' : `'${String(s).replace(/'/g, "''")}'`;

// ---------- instances ----------
const instances = [
  { id: 'demo-survival', name: 'Survival Hub', mc: '1.21.1', loader: 'fabric', lv: '0.16.9', biome: 'forest', seed: 1, play: 103, last: 1, pinned: 1, bg: 'emerald', sym: 'grass' },
  { id: 'demo-create', name: 'Create Factory', mc: '1.20.1', loader: 'forge', lv: '47.3.0', biome: 'cherry', seed: 2, play: 250, last: 5, pinned: 1, bg: 'rose', sym: 'copper' },
  { id: 'demo-shaders', name: 'Shader Playground', mc: '1.21.1', loader: 'fabric', lv: '0.16.9', biome: 'ocean', seed: 0, play: 59, last: 26, bg: 'indigo', sym: 'diamond' },
  { id: 'demo-sky', name: 'Skyblock Ascent', mc: '1.21.1', loader: 'neoforge', lv: '21.1.77', biome: 'end', seed: 1, play: 41, last: 50, bg: 'midnight', sym: 'amethyst' },
  { id: 'demo-nether', name: 'Hardcore Nether', mc: '1.21.4', loader: 'vanilla', lv: null, biome: 'nether', seed: 2, play: 18, last: 75, bg: 'sunset', sym: 'redstone' },
  { id: 'demo-frost', name: 'Frostbound', mc: '1.20.1', loader: 'quilt', lv: '0.26.4', biome: 'snow', seed: 0, play: 33, last: 120, bg: 'lime', sym: 'ice' },
  { id: 'demo-desert', name: 'Desert Nomads', mc: '1.21.1', loader: 'vanilla', lv: null, biome: 'desert', seed: 1, play: 12, last: 200, bg: 'amber', sym: 'sand' },
  { id: 'demo-mush', name: 'Mushroom Isles', mc: '1.20.4', loader: 'fabric', lv: '0.15.11', biome: 'mushroom', seed: 2, play: 7, last: 300, bg: 'rose', sym: 'moss' },
];

for (const i of instances) {
  const dir = path.join(INST, i.name.replace(/ /g, '-'));
  for (const sub of ['mods', 'saves', 'resourcepacks', 'shaderpacks', 'config', 'screenshots', 'logs']) fs.mkdirSync(path.join(dir, sub), { recursive: true });
  i.dir = dir;
  sql(`INSERT OR REPLACE INTO instances (id,name,mc_version,loader,loader_version,playtime_seconds,mod_count,last_played_at,installed_at,pinned,state,biome,art_seed,directory_name,icon_background,icon_symbol,version_id)
       VALUES (${q(i.id)},${q(i.name)},${q(i.mc)},${q(i.loader)},${q(i.lv)},${i.play * H},0,${now - i.last * H},${now - 40 * 24 * H},${i.pinned ? 1 : 0},'idle',${q(i.biome)},${i.seed},${q(path.basename(dir))},${q(i.bg)},${q(i.sym)},${q(i.mc)})`);
}

// ---------- worlds ----------
const worlds = {
  'demo-survival': [['Homestead', 0, 2, 412000, 1], ['Mega Base', 0, 3, 960000, 3], ['Creative Lab', 1, 1, 50000, 30]],
  'demo-create': [['Brass Works', 0, 2, 2200000, 5], ['Train Network', 1, 0, 300000, 60]],
  'demo-shaders': [['Coral Reef', 1, 1, 120000, 26]],
  'demo-sky': [['Void Island', 0, 2, 700000, 50]],
  'demo-nether': [['Inferno Run', 0, 3, 180000, 75]],
  'demo-frost': [['Glacier Peak', 0, 2, 380000, 120]],
};
const scenes = ['forest', 'cherry', 'ocean', 'end', 'nether', 'snow', 'desert', 'mushroom'];
let k = 0;
for (const [id, list] of Object.entries(worlds)) {
  const inst = instances.find((i) => i.id === id);
  for (const [name, mode, diff, ticks, hoursAgo] of list) {
    const wdir = path.join(inst.dir, 'saves', name);
    fs.mkdirSync(wdir, { recursive: true });
    fs.writeFileSync(path.join(wdir, 'level.dat'), nbtFile({
      Data: ['compound', {
        LevelName: ['string', name], GameType: ['int', mode], Difficulty: ['byte', diff], hardcore: ['byte', id === 'demo-nether' ? 1 : 0],
        LastPlayed: ['long', (now - hoursAgo * H) * 1000], Time: ['long', ticks], RandomSeed: ['long', -4172144997902289642n + BigInt(k * 7919)],
        Version: ['compound', { Name: ['string', inst.mc] }],
      }],
    }));
    // A 64px icon cut from the project's own scene artwork.
    const scene = path.join(REPO, 'res/artwork/scenes', `scene-${scenes[k % scenes.length]}.svg`);
    execFileSync('magick', ['-background', 'none', '-density', '96', scene, '-gravity', 'center', '-crop', '60%x100%+0+0', '+repage', '-resize', '64x64^', '-extent', '64x64', path.join(wdir, 'icon.png')]);
    fs.writeFileSync(path.join(wdir, 'region.bin'), Buffer.alloc(1024 * 1024 * (5 + k * 3)));
    k++;
  }
}
// A backup for one world
fs.mkdirSync(path.join(instances[0].dir, 'backups'), { recursive: true });

// ---------- mods from Modrinth ----------
async function fetchMods(inst, slugs) {
  const files = [];
  for (const slug of slugs) {
    const url = `https://api.modrinth.com/v2/project/${slug}/version?loaders=${encodeURIComponent(JSON.stringify([inst.loader]))}&game_versions=${encodeURIComponent(JSON.stringify([inst.mc]))}`;
    const res = await fetch(url, { headers: { 'User-Agent': 'lodestone-demo-seed' } });
    if (!res.ok) { console.error('skip', slug, res.status); continue; }
    const versions = await res.json();
    const v = versions[0]; if (!v) { console.error('none', slug); continue; }
    const f = v.files.find((x) => x.primary) || v.files[0];
    const target = path.join(inst.dir, 'mods', f.filename);
    if (!fs.existsSync(target)) {
      const bin = Buffer.from(await (await fetch(f.url)).arrayBuffer());
      fs.writeFileSync(target, bin);
    }
    const proj = await (await fetch(`https://api.modrinth.com/v2/project/${slug}`)).json();
    sql(`INSERT OR REPLACE INTO mod_sources (instance_id,file_name,platform,project_id,version_id,name,sha1,added_at,version_number)
         VALUES (${q(inst.id)},${q(f.filename)},'modrinth',${q(v.project_id)},${q(v.id)},${q(proj.title)},${q(f.hashes.sha1)},${now - 86400},${q(v.version_number)})`);
    files.push(f.filename);
    console.log(inst.name, '<-', f.filename);
  }
  sql(`UPDATE instances SET mod_count=${files.length} WHERE id=${q(inst.id)}`);
}

const fabricSet = ['fabric-api', 'sodium', 'lithium', 'iris', 'modmenu', 'ferrite-core', 'entityculling', 'appleskin', 'jade', 'mouse-tweaks', 'xaeros-minimap', 'continuity'];
await fetchMods(instances[0], fabricSet);
await fetchMods(instances[2], ['fabric-api', 'sodium', 'iris', 'continuity', 'modmenu']);
await fetchMods(instances[1], ['create', 'jei', 'jade', 'appleskin', 'mouse-tweaks', 'ferrite-core', 'create-crafts-and-additions', 'createaddition']);
await fetchMods(instances[3], ['jei', 'jade', 'appleskin', 'ferrite-core']);

// ---------- servers ----------
const servers = [['Hypixel', 'mc.hypixel.net', 1], ['Wynncraft', 'play.wynncraft.com', 1], ['CubeCraft', 'play.cubecraft.net', 0], ['2b2t', '2b2t.org', 0], ['Mineplex', 'us.mineplex.com', 0]];
for (const [n, a, f] of servers) sql(`INSERT OR IGNORE INTO servers (name,address,favorite,added_at) VALUES (${q(n)},${q(a)},${f},${now})`);
console.log('done');
