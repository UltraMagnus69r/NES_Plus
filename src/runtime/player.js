/**
 * Emberleaf play runtime.
 * Original platform physics for this project. It is not derived from any console game.
 * The workshop and the standalone HTML export both load this file.
 */

export const TILE = 16;

export const PHYSICS = {
  gravity: 0.4,
  maxFall: 5.4,
  runAccel: 0.32,
  maxRun: 2.25,
  friction: 0.74,
  airFriction: 0.985,
  jumpVelocity: -6.35,
  jumpCut: 0.48,
  coyoteFrames: 6,
  jumpBufferFrames: 7,
  stompBounce: -4.3,
  stompSlop: 8,
};

export function makeWorld(spec = {}) {
  const width = spec.width ?? 12;
  const height = spec.height ?? 8;
  const solids = spec.solids ?? new Uint8Array(width * height);
  const hazards = spec.hazards ?? new Uint8Array(width * height);
  const player = {
    x: 16,
    y: 16,
    vx: 0,
    vy: 0,
    w: 12,
    h: 14,
    onGround: false,
    facing: 1,
    coyote: 0,
    jumpBuffer: 0,
    alive: true,
    hurtCooldown: 0,
    ...(spec.player || {}),
  };
  const actors = (spec.actors ?? []).map((a, i) => ({
    id: a.id ?? `actor-${i}`,
    assetId: a.assetId ?? '',
    x: a.x ?? 0,
    y: a.y ?? 0,
    w: a.w ?? 14,
    h: a.h ?? 14,
    dir: a.dir ?? -1,
    speed: a.speed ?? 0.55,
    behavior: a.behavior ?? 'patrol',
    alive: a.alive !== false,
  }));
  return {
    tileSize: TILE,
    width,
    height,
    solids,
    hazards,
    actors,
    initialActors: actors.map((a) => ({ ...a })),
    player,
    spawn: { x: player.x, y: player.y },
    completed: false,
    deadTimer: 0,
    events: [],
    ...PHYSICS,
    ...(spec.tuning || {}),
  };
}

export function isSolidTile(world, tx, ty) {
  if (ty < 0 || ty >= world.height) return false;
  if (tx < 0 || tx >= world.width) return true;
  return world.solids[ty * world.width + tx] === 1;
}

function overlappingSolids(world, x, y, w, h) {
  const x0 = Math.floor(x / TILE);
  const y0 = Math.floor(y / TILE);
  const x1 = Math.floor((x + w - 0.001) / TILE);
  const y1 = Math.floor((y + h - 0.001) / TILE);
  const hits = [];
  for (let ty = y0; ty <= y1; ty += 1) {
    for (let tx = x0; tx <= x1; tx += 1) {
      if (isSolidTile(world, tx, ty)) hits.push({ tx, ty });
    }
  }
  return hits;
}

function moveX(world, body) {
  body.x += body.vx;
  const hits = overlappingSolids(world, body.x, body.y, body.w, body.h);
  if (!hits.length) return;
  if (body.vx > 0) {
    let min = Infinity;
    for (const hit of hits) min = Math.min(min, hit.tx * TILE);
    body.x = min - body.w;
  } else if (body.vx < 0) {
    let max = -Infinity;
    for (const hit of hits) max = Math.max(max, (hit.tx + 1) * TILE);
    body.x = max;
  }
  body.vx = 0;
}

function moveY(world, body) {
  body.onGround = false;
  body.y += body.vy;
  const hits = overlappingSolids(world, body.x, body.y, body.w, body.h);
  if (hits.length) {
    if (body.vy >= 0) {
      let min = Infinity;
      for (const hit of hits) min = Math.min(min, hit.ty * TILE);
      body.y = min - body.h;
      body.onGround = true;
    } else {
      let max = -Infinity;
      for (const hit of hits) max = Math.max(max, (hit.ty + 1) * TILE);
      body.y = max;
    }
    body.vy = 0;
    return;
  }
  if (body.vy >= 0 && overlappingSolids(world, body.x, body.y + 0.6, body.w, body.h).length) {
    body.onGround = true;
  }
}

function overlap(a, b) {
  return a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y;
}

function stepPatrol(world, actor) {
  if (actor.behavior !== 'patrol' || !actor.alive) return;
  const dir = actor.dir >= 0 ? 1 : -1;
  const speed = actor.speed || 0;
  const next = actor.x + dir * speed;
  const frontX = dir > 0 ? next + actor.w + 0.01 : next - 0.01;
  const footY = actor.y + actor.h - 1;
  const tx = Math.floor(frontX / TILE);
  const ty = Math.floor(footY / TILE);
  if (isSolidTile(world, tx, ty) || !isSolidTile(world, tx, ty + 1)) {
    actor.dir = -dir;
    return;
  }
  actor.x = next;
}

function touchesHazard(world, body) {
  const x0 = Math.floor(body.x / TILE);
  const y0 = Math.floor(body.y / TILE);
  const x1 = Math.floor((body.x + body.w - 0.001) / TILE);
  const y1 = Math.floor((body.y + body.h - 0.001) / TILE);
  for (let ty = y0; ty <= y1; ty += 1) {
    for (let tx = x0; tx <= x1; tx += 1) {
      if (tx < 0 || ty < 0 || tx >= world.width || ty >= world.height) continue;
      if (world.hazards[ty * world.width + tx]) return true;
    }
  }
  return false;
}

function hurt(world) {
  const p = world.player;
  if (!p.alive || p.hurtCooldown > 0) return;
  p.alive = false;
  p.vx = 0;
  p.vy = 0;
  world.deadTimer = 28;
  world.events.push('hurt');
}

function respawn(world) {
  const p = world.player;
  p.x = world.spawn.x;
  p.y = world.spawn.y;
  p.vx = 0;
  p.vy = 0;
  p.alive = true;
  p.onGround = false;
  p.coyote = 0;
  p.jumpBuffer = 0;
  p.hurtCooldown = 20;
  world.actors = world.initialActors.map((a) => ({ ...a }));
}

export function stepWorld(world, input = {}) {
  world.events = [];
  if (world.completed) return world;
  const p = world.player;
  if (!p.alive) {
    world.deadTimer -= 1;
    if (world.deadTimer <= 0) respawn(world);
    return world;
  }
  if (p.hurtCooldown > 0) p.hurtCooldown -= 1;

  if (p.onGround) p.coyote = world.coyoteFrames;
  else if (p.coyote > 0) p.coyote -= 1;

  if (input.jumpPressed) p.jumpBuffer = world.jumpBufferFrames;
  else if (p.jumpBuffer > 0) p.jumpBuffer -= 1;

  if (p.jumpBuffer > 0 && (p.onGround || p.coyote > 0)) {
    p.vy = world.jumpVelocity;
    p.onGround = false;
    p.coyote = 0;
    p.jumpBuffer = 0;
    world.events.push('jump');
  } else if (input.jumpReleased && p.vy < 0) {
    p.vy *= world.jumpCut;
  }

  if (input.left && !input.right) {
    p.vx -= world.runAccel;
    p.facing = -1;
  } else if (input.right && !input.left) {
    p.vx += world.runAccel;
    p.facing = 1;
  } else if (p.onGround) {
    p.vx *= world.friction;
    if (Math.abs(p.vx) < 0.04) p.vx = 0;
  } else {
    p.vx *= world.airFriction;
  }
  if (p.vx > world.maxRun) p.vx = world.maxRun;
  if (p.vx < -world.maxRun) p.vx = -world.maxRun;

  p.vy += world.gravity;
  if (p.vy > world.maxFall) p.vy = world.maxFall;

  moveX(world, p);
  const appliedVy = p.vy;
  moveY(world, p);
  const prevBottom = p.y + p.h - appliedVy;

  for (const actor of world.actors) stepPatrol(world, actor);

  for (const actor of world.actors) {
    if (!actor.alive || !overlap(p, actor)) continue;
    if (actor.behavior === 'goal') {
      world.completed = true;
      world.events.push('goal');
      return world;
    }
  }

  for (const actor of world.actors) {
    if (!actor.alive || !overlap(p, actor)) continue;
    if (actor.behavior === 'collect') {
      actor.alive = false;
      world.events.push('collect');
      continue;
    }
    if (actor.behavior === 'patrol') {
      const stomping = appliedVy > 0 && prevBottom <= actor.y + world.stompSlop;
      if (stomping) {
        actor.alive = false;
        p.vy = world.stompBounce;
        p.onGround = false;
        world.events.push('stomp');
      } else hurt(world);
      continue;
    }
    if (actor.behavior === 'hurt') hurt(world);
  }

  if (p.alive && (touchesHazard(world, p) || p.y > world.height * TILE + 24)) hurt(world);
  return world;
}

export function buildWorld(level, assets) {
  const byId = {};
  for (const asset of assets) byId[asset.id] = asset;
  const solids = new Uint8Array(level.w * level.h);
  const hazards = new Uint8Array(level.w * level.h);
  const actors = [];
  for (const cell of level.cells || []) {
    const asset = byId[cell.assetId];
    if (!asset || cell.x < 0 || cell.y < 0 || cell.x >= level.w || cell.y >= level.h) continue;
    const behavior = asset.behavior || 'decoration';
    const index = cell.y * level.w + cell.x;
    if (cell.layer === 'main' && asset.solid) solids[index] = 1;
    if (cell.layer === 'main' && behavior === 'hurt' && asset.solid) hazards[index] = 1;
    const actorLike = behavior === 'patrol' || behavior === 'collect' || behavior === 'goal' || (behavior === 'hurt' && !asset.solid);
    if (!actorLike) continue;
    const w = 14;
    const h = 14;
    actors.push({
      id: `${cell.x}:${cell.y}:${cell.assetId}`,
      assetId: cell.assetId,
      x: cell.x * TILE + (TILE - w) / 2,
      y: (cell.y + 1) * TILE - h,
      w,
      h,
      dir: cell.x % 2 === 0 ? -1 : 1,
      speed: behavior === 'patrol' ? 0.55 : 0,
      behavior,
      alive: true,
    });
  }
  const world = makeWorld({
    width: level.w,
    height: level.h,
    solids,
    hazards,
    actors,
    player: {
      x: level.player.x * TILE + 2,
      y: (level.player.y + 1) * TILE - 14,
      w: 12,
      h: 14,
    },
  });
  const p = world.player;
  const tx = Math.floor((p.x + p.w / 2) / TILE);
  const ty = Math.floor((p.y + p.h) / TILE);
  p.onGround = isSolidTile(world, tx, ty);
  world.spawn = { x: p.x, y: p.y };
  return world;
}

const VIEW_W = 320;
const VIEW_H = 180;

function playerAsset(project) {
  if (project.playerId) {
    const chosen = project.assets.find((a) => a.id === project.playerId);
    if (chosen) return chosen;
  }
  return project.assets.find((a) => a.kind === 'player') || null;
}

function tone(ctx, freq, dur, type, gain, when) {
  const t = when ?? ctx.currentTime;
  const osc = ctx.createOscillator();
  const amp = ctx.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(freq, t);
  amp.gain.setValueAtTime(gain, t);
  amp.gain.exponentialRampToValueAtTime(0.0001, t + dur);
  osc.connect(amp);
  amp.connect(ctx.destination);
  osc.start(t);
  osc.stop(t + dur + 0.02);
}

const THEME = [
  [0, 294, 0.45], [0.5, 349, 0.45], [1, 392, 0.45], [1.5, 440, 0.45],
  [2, 392, 0.45], [2.5, 349, 0.45], [3, 330, 0.45], [3.5, 294, 0.9],
  [4.5, 440, 0.45], [5, 523, 0.45], [5.5, 587, 0.45], [6, 523, 0.45],
  [6.5, 440, 0.45], [7, 392, 0.45], [7.5, 349, 0.9],
];

function playPreset(ctx, preset) {
  if (!ctx) return;
  if (preset === 'jump') tone(ctx, 520, 0.12, 'square', 0.05);
  else if (preset === 'stomp') tone(ctx, 180, 0.1, 'square', 0.06);
  else if (preset === 'collect') {
    tone(ctx, 660, 0.08, 'square', 0.04);
    tone(ctx, 880, 0.12, 'square', 0.04, ctx.currentTime + 0.07);
  } else if (preset === 'hurt') tone(ctx, 140, 0.22, 'sawtooth', 0.05);
  else if (preset === 'goal') {
    tone(ctx, 392, 0.12, 'square', 0.05);
    tone(ctx, 523, 0.12, 'square', 0.05, ctx.currentTime + 0.1);
    tone(ctx, 659, 0.22, 'square', 0.05, ctx.currentTime + 0.2);
  }
}

export function previewSound(preset) {
  const Ctx = window.AudioContext || window.webkitAudioContext;
  if (!Ctx) return;
  const ctx = new Ctx();
  playPreset(ctx, preset);
}

export function runGame(canvas, project, options = {}) {
  const ctx = canvas.getContext('2d');
  const off = document.createElement('canvas');
  off.width = VIEW_W;
  off.height = VIEW_H;
  const octx = off.getContext('2d');
  let dead = false;
  let raf = 0;
  const keys = { left: false, right: false, jump: false };
  let prevJump = false;
  let prevLeft = false;
  let prevRight = false;
  let audioCtx = null;
  let musicGain = null;
  let musicTimer = 0;
  let musicGen = 0;
  const buffers = new Map();
  const images = new Map();
  let mode = options.courseId ? 'level' : 'map';
  let level = null;
  let world = null;
  let cleared = new Set();
  let selected = project.startNodeId;
  let seeds = 0;
  let banner = '';
  let bannerTimer = 0;
  let won = false;
  let fit = { ox: 0, oy: 0, scale: 1, cssW: 1, cssH: 1 };
  const touchRoot = document.createElement('div');
  const backBtn = document.createElement('button');

  function ac() {
    const Ctx = window.AudioContext || window.webkitAudioContext;
    if (!Ctx) return null;
    if (!audioCtx) audioCtx = new Ctx();
    if (audioCtx.state === 'suspended') audioCtx.resume();
    return audioCtx;
  }

  function cueAsset(name) {
    const matches = project.assets.filter((a) => a.kind === 'sfx' && a.preset === name);
    return matches.find((a) => a.src) || matches[0];
  }

  function playCue(name) {
    const ctxNow = ac();
    if (!ctxNow) return;
    const asset = cueAsset(name);
    if (asset && asset.src && buffers.has(asset.id)) {
      const src = ctxNow.createBufferSource();
      src.buffer = buffers.get(asset.id);
      src.connect(ctxNow.destination);
      src.start();
      return;
    }
    playPreset(ctxNow, name);
  }

  function stopMusic() {
    musicGen += 1;
    window.clearTimeout(musicTimer);
    if (musicGain && audioCtx) musicGain.gain.setValueAtTime(0, audioCtx.currentTime);
  }

  function startTheme(asset) {
    stopMusic();
    const ctxNow = ac();
    if (!ctxNow) return;
    const gen = musicGen;
    musicGain = ctxNow.createGain();
    musicGain.gain.value = 0.9;
    musicGain.connect(ctxNow.destination);
    if (asset && asset.src && buffers.has(asset.id)) {
      const loop = () => {
        if (gen !== musicGen) return;
        const src = ctxNow.createBufferSource();
        src.buffer = buffers.get(asset.id);
        src.connect(musicGain);
        src.onended = loop;
        src.start();
      };
      loop();
      return;
    }
    const bpm = 112;
    const beat = 60 / bpm;
    const loopBeats = 9.2;
    const schedule = (startAt) => {
      if (gen !== musicGen) return;
      for (const [b, freq, dur] of THEME) {
        tone(ctxNow, freq, dur * beat * 0.9, 'square', 0.035, startAt + b * beat);
        tone(ctxNow, freq / 2, dur * beat * 0.9, 'triangle', 0.03, startAt + b * beat);
      }
      const next = startAt + loopBeats * beat;
      const wait = Math.max(0, (next - ctxNow.currentTime - 0.2) * 1000);
      musicTimer = window.setTimeout(() => schedule(next), wait);
    };
    schedule(ctxNow.currentTime + 0.05);
  }

  function levelById(id) {
    return project.levels.find((item) => item.id === id) || project.levels[0];
  }

  function nodeById(id) {
    return project.nodes.find((node) => node.id === id) || project.nodes[0];
  }

  function reachable() {
    const seen = new Set();
    const queue = [project.startNodeId];
    const byId = new Map(project.nodes.map((node) => [node.id, node]));
    while (queue.length) {
      const id = queue.pop();
      if (!id || seen.has(id) || !byId.has(id)) continue;
      seen.add(id);
      for (const link of byId.get(id).links || []) queue.push(link);
    }
    return seen;
  }

  function unlocked() {
    const open = new Set([project.startNodeId]);
    let changed = true;
    const byId = new Map(project.nodes.map((node) => [node.id, node]));
    while (changed) {
      changed = false;
      for (const id of [...open]) {
        const node = byId.get(id);
        if (!node || !cleared.has(node.levelId)) continue;
        for (const link of node.links || []) {
          if (!open.has(link) && byId.has(link)) {
            open.add(link);
            changed = true;
          }
        }
      }
    }
    return open;
  }

  function checkWin() {
    const reach = reachable();
    won = [...reach].every((id) => {
      const node = nodeById(id);
      return node && cleared.has(node.levelId);
    });
    if (won) banner = 'Every linked course is clear.';
  }

  function enterLevel(id) {
    level = levelById(id);
    world = buildWorld(level, project.assets);
    seeds = 0;
    mode = 'level';
    const music = project.assets.find((a) => a.id === level.musicId);
    if (music) startTheme(music);
    else stopMusic();
  }

  function onKeyDown(event) {
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement) return;
    const k = event.key.toLowerCase();
    if (k === 'arrowleft' || k === 'a') keys.left = true;
    if (k === 'arrowright' || k === 'd') keys.right = true;
    if (k === 'arrowup' || k === 'w' || k === ' ' || k === 'z') keys.jump = true;
    if (['arrowleft', 'arrowright', 'arrowup', ' ', 'z'].includes(event.key.toLowerCase()) || event.key === ' ') event.preventDefault();
    if (k === 'escape') leave();
    ac();
  }
  function onKeyUp(event) {
    const k = event.key.toLowerCase();
    if (k === 'arrowleft' || k === 'a') keys.left = false;
    if (k === 'arrowright' || k === 'd') keys.right = false;
    if (k === 'arrowup' || k === 'w' || k === ' ' || k === 'z') keys.jump = false;
  }

  function leave() {
    if (mode === 'level' && !options.courseId) {
      mode = 'map';
      stopMusic();
      return;
    }
    if (options.onExit) options.onExit();
  }

  function pointerToView(event) {
    const rect = canvas.getBoundingClientRect();
    const x = (event.clientX - rect.left - fit.ox) / fit.scale;
    const y = (event.clientY - rect.top - fit.oy) / fit.scale;
    return { x, y };
  }

  function onPointer(event) {
    ac();
    if (mode !== 'map') return;
    const p = pointerToView(event);
    const open = unlocked();
    for (const node of project.nodes) {
      const dx = p.x - node.x;
      const dy = p.y - node.y;
      if (dx * dx + dy * dy < 18 * 18) {
        selected = node.id;
        if (open.has(node.id)) enterLevel(node.levelId);
        else banner = 'Clear a linked course first.';
      }
    }
  }

  function readInput() {
    const jump = keys.jump;
    const input = {
      left: keys.left,
      right: keys.right,
      jump,
      jumpPressed: jump && !prevJump,
      jumpReleased: !jump && prevJump,
    };
    prevJump = jump;
    return input;
  }

  function drawImage(img, x, y, w, h, flip) {
    if (!img) return;
    if (!flip) {
      octx.drawImage(img, Math.round(x), Math.round(y), w, h);
      return;
    }
    octx.save();
    octx.translate(Math.round(x + w), Math.round(y));
    octx.scale(-1, 1);
    octx.drawImage(img, 0, 0, w, h);
    octx.restore();
  }

  function drawLevel() {
    const camX = Math.max(0, Math.min(level.w * TILE - VIEW_W, Math.round(world.player.x + world.player.w / 2 - VIEW_W / 2)));
    const camY = Math.max(0, Math.min(level.h * TILE - VIEW_H, Math.round(world.player.y + world.player.h / 2 - VIEW_H / 2)));
    octx.fillStyle = level.sky || '#8fb8c9';
    octx.fillRect(0, 0, VIEW_W, VIEW_H);
    octx.fillStyle = 'rgba(62, 83, 64, 0.45)';
    const base = 58;
    for (let i = -1; i < 8; i += 1) {
      const x = i * 70 - Math.floor(camX * 0.25) % 70;
      octx.beginPath();
      octx.moveTo(x, base + 40);
      octx.lineTo(x + 34, base - 8);
      octx.lineTo(x + 70, base + 40);
      octx.fill();
    }
    const para = Math.round(camX * 0.45);
    const drawLayer = (layer, ox) => {
      for (const cell of level.cells) {
        if (cell.layer !== layer) continue;
        const asset = project.assets.find((a) => a.id === cell.assetId);
        if (!asset) continue;
        if (layer === 'main' && (asset.behavior === 'patrol' || asset.behavior === 'collect' || asset.behavior === 'goal' || (asset.behavior === 'hurt' && !asset.solid))) continue;
        const img = images.get(asset.id);
        const x = cell.x * TILE - ox;
        const y = cell.y * TILE - (layer === 'back' ? Math.round(camY * 0.45) : camY);
        if (x > VIEW_W || y > VIEW_H || x < -TILE || y < -TILE) continue;
        if (img) drawImage(img, x, y, TILE, TILE, false);
      }
    };
    drawLayer('back', para);
    drawLayer('main', camX);
    for (const actor of world.actors) {
      if (!actor.alive) continue;
      const img = images.get(actor.assetId);
      drawImage(img, actor.x - 1 - camX, actor.y - 2 - camY, TILE, TILE, actor.dir < 0);
    }
    const hero = playerAsset(project);
    const bob = world.player.onGround && Math.abs(world.player.vx) > 0.2 ? (Math.floor(performance.now() / 120) % 2) : 0;
    if (hero && world.player.alive) {
      drawImage(images.get(hero.id), world.player.x - 2 - camX, world.player.y - 2 - camY - bob, TILE, TILE, world.player.facing < 0);
    }
    drawLayer('front', camX);
    octx.fillStyle = '#1a1612';
    octx.fillRect(8, 8, 150, 18);
    octx.fillStyle = '#efe6d6';
    octx.font = '12px system-ui, sans-serif';
    octx.fillText(`${level.name}   seeds ${seeds}`, 12, 21);
    if (!world.player.alive) {
      octx.fillStyle = '#e25b2a';
      octx.fillText('Back to the start…', 12, 40);
    }
  }

  function drawMap() {
    octx.fillStyle = '#171512';
    octx.fillRect(0, 0, VIEW_W, VIEW_H);
    octx.fillStyle = '#3e5340';
    for (const node of project.nodes) {
      for (const link of node.links) {
        const other = nodeById(link);
        if (!other || other.id < node.id) continue;
        octx.beginPath();
        octx.moveTo(node.x, node.y);
        octx.lineTo(other.x, other.y);
        octx.strokeStyle = '#7d9a62';
        octx.lineWidth = 3;
        octx.stroke();
      }
    }
    const open = unlocked();
    for (const node of project.nodes) {
      const isSel = node.id === selected;
      const isClear = cleared.has(node.levelId);
      octx.beginPath();
      octx.arc(node.x, node.y, isSel ? 12 : 10, 0, Math.PI * 2);
      octx.fillStyle = !open.has(node.id) ? '#2e2a24' : isClear ? '#7d9a62' : '#e25b2a';
      octx.fill();
      if (isSel) {
        octx.strokeStyle = '#e4b15a';
        octx.lineWidth = 2;
        octx.stroke();
      }
      octx.fillStyle = '#efe6d6';
      octx.font = '12px system-ui, sans-serif';
      octx.fillText(node.name, node.x + 16, node.y + 4);
    }
    octx.fillStyle = '#efe6d6';
    octx.font = '14px Georgia, serif';
    octx.fillText(project.name || 'Emberleaf', 12, 22);
    octx.font = '12px system-ui, sans-serif';
    if (won) octx.fillText('Every linked course is clear.', 12, VIEW_H - 14);
    else {
      octx.fillText('Jump enters the lit course.', 12, VIEW_H - 28);
      octx.fillText('Clear it and the next path opens.', 12, VIEW_H - 12);
    }
  }

  function frame(now) {
    if (dead) return;
    const leftEdge = keys.left && !prevLeft;
    const rightEdge = keys.right && !prevRight;
    const input = readInput();
    prevLeft = keys.left;
    prevRight = keys.right;
    if (mode === 'map') {
      if (input.jumpPressed) {
        const open = unlocked();
        const node = nodeById(selected);
        if (node && open.has(node.id)) enterLevel(node.levelId);
        else banner = 'Clear a linked course first.';
      }
      if (leftEdge || rightEdge) {
        const node = nodeById(selected);
        const open = unlocked();
        const links = (node?.links || []).filter((id) => open.has(id));
        if (links.length) selected = links[0];
      }
    } else if (mode === 'level' && world) {
      stepWorld(world, input);
      for (const event of world.events) {
        if (event === 'collect') seeds += 1;
        if (event === 'jump' || event === 'stomp' || event === 'collect' || event === 'hurt' || event === 'goal') playCue(event);
      }
      if (world.completed) {
        cleared.add(level.id);
        checkWin();
        banner = won ? 'Every linked course is clear.' : `${level.name} clear`;
        bannerTimer = 90;
        if (options.courseId) {
          mode = 'done';
          stopMusic();
        } else {
          mode = 'map';
          stopMusic();
          const node = project.nodes.find((item) => item.levelId === level.id);
          if (node) selected = node.id;
        }
        if (options.onClear) options.onClear(level.id);
      }
    }
    if (bannerTimer > 0) bannerTimer -= 1;
    octx.imageSmoothingEnabled = false;
    if (mode === 'level' && world && level) drawLevel();
    else if (mode === 'done') {
      octx.fillStyle = '#12110f';
      octx.fillRect(0, 0, VIEW_W, VIEW_H);
      octx.fillStyle = '#efe6d6';
      octx.font = '20px Georgia, serif';
      octx.fillText('Course clear', 24, 80);
      octx.font = '13px system-ui, sans-serif';
      octx.fillText('Back returns to the workshop.', 24, 108);
    } else drawMap();
    if (banner && (mode !== 'level' || bannerTimer > 0)) {
      octx.fillStyle = 'rgba(26, 22, 18, 0.85)';
      octx.fillRect(12, VIEW_H - 36, Math.min(296, 8 + banner.length * 7), 20);
      octx.fillStyle = '#e4b15a';
      octx.font = '12px system-ui, sans-serif';
      octx.fillText(banner, 18, VIEW_H - 22);
    }

    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    const bw = Math.max(1, Math.round(rect.width * dpr));
    const bh = Math.max(1, Math.round(rect.height * dpr));
    if (canvas.width !== bw || canvas.height !== bh) {
      canvas.width = bw;
      canvas.height = bh;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.imageSmoothingEnabled = false;
    ctx.fillStyle = '#12110f';
    ctx.fillRect(0, 0, rect.width, rect.height);
    let scale = Math.min(rect.width / VIEW_W, rect.height / VIEW_H);
    const integer = Math.floor(scale);
    if (integer >= 1) scale = integer;
    const dw = VIEW_W * scale;
    const dh = VIEW_H * scale;
    const ox = Math.floor((rect.width - dw) / 2);
    const oy = Math.floor((rect.height - dh) / 2);
    fit = { ox, oy, scale, cssW: rect.width, cssH: rect.height };
    ctx.drawImage(off, ox, oy, dw, dh);
    raf = requestAnimationFrame(frame);
    void now;
  }

  function bindTouch(button, key) {
    const down = (event) => {
      event.preventDefault();
      keys[key] = true;
      ac();
    };
    const up = (event) => {
      event.preventDefault();
      keys[key] = false;
    };
    button.addEventListener('pointerdown', down);
    button.addEventListener('pointerup', up);
    button.addEventListener('pointercancel', up);
    button.addEventListener('pointerleave', up);
  }

  function mountChrome() {
    const coarse = window.matchMedia('(pointer: coarse)').matches || navigator.maxTouchPoints > 0;
    touchRoot.style.cssText = 'position:absolute;inset:auto 12px 12px 12px;display:flex;justify-content:space-between;gap:12px;pointer-events:none;z-index:3;';
    if (coarse || options.touch === true) {
      const cluster = document.createElement('div');
      cluster.style.cssText = 'display:flex;gap:8px;pointer-events:auto;';
      const jump = document.createElement('button');
      jump.textContent = 'Jump';
      const style = 'min-width:72px;min-height:56px;border-radius:16px;border:1px solid #2e2a24;background:#1c1a17;color:#efe6d6;font:600 14px/1 system-ui,sans-serif;';
      for (const [label, key] of [['Left', 'left'], ['Right', 'right']]) {
        const button = document.createElement('button');
        button.type = 'button';
        button.textContent = label;
        button.style.cssText = style;
        bindTouch(button, key);
        cluster.appendChild(button);
      }
      jump.type = 'button';
      jump.style.cssText = style + 'background:#e25b2a;color:#1a1612;min-width:96px;pointer-events:auto;';
      bindTouch(jump, 'jump');
      touchRoot.append(cluster, jump);
    }
    const host = canvas.parentElement || document.body;
    if (getComputedStyle(host).position === 'static') host.style.position = 'relative';
    host.appendChild(touchRoot);
    if (options.onExit) {
      backBtn.type = 'button';
      backBtn.textContent = 'Back';
      backBtn.style.cssText = 'position:absolute;top:12px;right:12px;z-index:4;border-radius:999px;border:1px solid #2e2a24;background:#1c1a17;color:#efe6d6;padding:8px 12px;';
      backBtn.addEventListener('click', leave);
      host.appendChild(backBtn);
    }
  }

  window.addEventListener('keydown', onKeyDown);
  window.addEventListener('keyup', onKeyUp);
  canvas.addEventListener('pointerdown', onPointer);
  mountChrome();

  const boot = async () => {
    await Promise.all(project.assets.filter((a) => a.src && a.kind !== 'sfx' && a.kind !== 'music').map((asset) => new Promise((resolve) => {
      const img = new Image();
      img.onload = () => {
        images.set(asset.id, img);
        resolve();
      };
      img.onerror = () => resolve();
      img.src = asset.src;
    })));
    const ctxNow = ac();
    if (ctxNow) {
      await Promise.all(project.assets.filter((a) => a.src && (a.kind === 'sfx' || a.kind === 'music')).map(async (asset) => {
        try {
          const res = await fetch(asset.src);
          const raw = await res.arrayBuffer();
          buffers.set(asset.id, await ctxNow.decodeAudioData(raw.slice(0)));
        } catch {
          /* preset sounds still play */
        }
      }));
    }
    if (dead) return;
    if (options.courseId) enterLevel(options.courseId);
    raf = requestAnimationFrame(frame);
  };
  boot();

  return () => {
    dead = true;
    cancelAnimationFrame(raf);
    window.clearTimeout(musicTimer);
    stopMusic();
    window.removeEventListener('keydown', onKeyDown);
    window.removeEventListener('keyup', onKeyUp);
    canvas.removeEventListener('pointerdown', onPointer);
    touchRoot.remove();
    backBtn.remove();
  };
}
