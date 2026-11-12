import { describe, expect, it } from 'vitest';
import { decodeBrr, decodeDpcmLevels } from './rom/audio';
import { suggestKind } from './rom/classify';
import { decodeTile, encodeGbTile, encodeNesTile } from './rom/codecs';
import { parseInes } from './rom/detect';
import { buildWorld, isSolidTile, makeWorld, stepWorld } from './runtime/player.js';
import { createDefaultProject } from './workshop/project';

describe('tiles and headers', () => {
  it('roundtrips NES 2bpp and reads iNES sizes including a trainer', () => {
    const pixels = new Uint8Array(64);
    for (let i = 0; i < 64; i += 1) pixels[i] = (i * 7 + 3) & 3;
    const encoded = encodeNesTile(pixels);
    expect(encoded.length).toBe(16);
    const decoded = decodeTile(encoded, 0, 'nes');
    expect([...(decoded ?? [])]).toEqual([...pixels]);

    const prg = new Uint8Array(16384).fill(0x11);
    const chr = new Uint8Array(8192).fill(0x22);
    const trainer = new Uint8Array(512).fill(0x33);
    const rom = new Uint8Array(16 + 512 + prg.length + chr.length);
    rom.set([0x4e, 0x45, 0x53, 0x1a, 1, 1, 0x04, 0], 0);
    rom.set(trainer, 16);
    rom.set(prg, 16 + 512);
    rom.set(chr, 16 + 512 + prg.length);
    const parsed = parseInes(rom);
    expect(parsed).not.toBeNull();
    expect(parsed?.trainerBytes).toBe(512);
    expect(parsed?.prgBytes).toBe(16384);
    expect(parsed?.chrBytes).toBe(8192);
    expect(parsed?.prg[0]).toBe(0x11);
    expect(parsed?.chr[0]).toBe(0x22);
    expect(parsed?.trainer?.[0]).toBe(0x33);
    expect(parsed?.chrRam).toBe(false);

    const ram = new Uint8Array(16 + 16384);
    ram.set([0x4e, 0x45, 0x53, 0x1a, 1, 0, 0, 0], 0);
    const noChr = parseInes(ram);
    expect(noChr?.chrBytes).toBe(0);
    expect(noChr?.chrRam).toBe(true);
    expect(noChr?.prgBytes).toBe(16384);
  });

  it('decodes Game Boy interleaved 2bpp', () => {
    const corner = new Uint8Array(64);
    corner[0] = 3;
    const encoded = encodeGbTile(corner);
    expect(encoded[0] & 0x80).toBe(0x80);
    expect(encoded[1] & 0x80).toBe(0x80);
    expect(encoded[8]).toBe(0);
    expect(decodeTile(encoded, 0, 'gb')?.[0]).toBe(3);
    const pixels = new Uint8Array(64);
    for (let i = 0; i < 64; i += 1) pixels[i] = (i * 3) & 3;
    const decoded = decodeTile(encodeGbTile(pixels), 0, 'gb');
    expect([...(decoded ?? [])]).toEqual([...pixels]);
  });
});

describe('audio', () => {
  it('decodes a known NES DPCM pattern', () => {
    expect(decodeDpcmLevels(new Uint8Array([0x01]))).toEqual([66, 64, 62, 60, 58, 56, 54, 52]);
  });

  it('decodes a quiet BRR filter 0 block without throwing', () => {
    const quiet = new Uint8Array(9);
    const nonzero = new Uint8Array(9);
    nonzero[0] = 0x10;
    nonzero[1] = 0x10;
    const loudShift = new Uint8Array(9);
    loudShift[0] = 0xf0;
    expect(decodeBrr(quiet).every((sample) => sample === 0)).toBe(true);
    const samples = decodeBrr(nonzero);
    expect(samples[0]).not.toBe(0);
    expect(Math.abs(samples[0])).toBeLessThan(0.01);
    expect(() => decodeBrr(loudShift)).not.toThrow();
  });
});

describe('suggestions and physics', () => {
  it('calls blank tiles empty and a centered blob a sprite', () => {
    expect(suggestKind(new Uint8Array(64))).toBe('empty');
    const blob = new Uint8Array(64);
    for (let y = 2; y <= 5; y += 1) {
      for (let x = 2; x <= 5; x += 1) blob[y * 8 + x] = 2;
    }
    expect(suggestKind(blob)).toBe('sprite');
  });

  it('lands the player on solid ground', () => {
    const solids = new Uint8Array(8 * 8);
    for (let x = 0; x < 8; x += 1) solids[6 * 8 + x] = 1;
    const world = stepWorld(
      makeWorld({ width: 8, height: 8, solids, player: { x: 20, y: 16, vx: 0, vy: 0, w: 12, h: 14 } }),
      {},
    );
    for (let i = 0; i < 180; i += 1) stepWorld(world, {});
    expect(world.player.onGround).toBe(true);
    expect(world.player.vy).toBe(0);
    expect(world.player.y + world.player.h).toBe(6 * 16);
  });

  it('removes a patrol enemy stomped from above', () => {
    const solids = new Uint8Array(10 * 10);
    for (let x = 0; x < 10; x += 1) solids[8 * 10 + x] = 1;
    const enemyY = 8 * 16 - 14;
    const world = makeWorld({
      width: 10,
      height: 10,
      solids,
      player: { x: 48, y: enemyY - 24, vx: 0, vy: 1, w: 12, h: 14 },
      actors: [{ id: 'mite', assetId: 'mite', x: 48, y: enemyY, w: 12, h: 14, dir: 1, speed: 0, behavior: 'patrol', alive: true }],
    });
    let removed = false;
    for (let i = 0; i < 40; i += 1) {
      stepWorld(world, {});
      if (!world.actors[0]?.alive) removed = true;
    }
    expect(removed).toBe(true);
  });

  it('lets a running jump clear both sample courses', () => {
    const project = createDefaultProject();
    for (const level of project.levels) {
      const world = buildWorld(level, project.assets);
      let hold = 0;
      let held = false;
      for (let frame = 0; frame < 4500 && !world.completed; frame += 1) {
        const p = world.player;
        const front = Math.floor((p.x + p.w + 4) / 16);
        const feet = Math.floor((p.y + p.h - 1) / 16);
        const solid = (tx: number, ty: number) => isSolidTile(world, tx, ty);
        let jump = false;
        if (p.onGround && (!solid(front, feet + 1) || solid(front, feet))) jump = true;
        for (const actor of world.actors) {
          if (!actor.alive) continue;
          const threat = actor.behavior === 'patrol' || actor.behavior === 'hurt';
          if (threat && actor.x < p.x + p.w + 36 && actor.x + actor.w > p.x + 4 && Math.abs(actor.y - p.y) < 24) jump = true;
        }
        if (p.onGround && Math.abs(p.vx) < 0.08 && frame > 15) jump = true;
        const pressing = jump || hold > 0;
        if (jump) hold = 16;
        else if (hold > 0) hold -= 1;
        stepWorld(world, {
          right: true,
          jump: pressing,
          jumpPressed: pressing && !held,
          jumpReleased: !pressing && held,
        });
        held = pressing;
      }
      expect(world.completed, level.name).toBe(true);
    }
  });
});
