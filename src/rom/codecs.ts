import type { CodecId } from '../types';

export function tileBytes(codec: CodecId): number {
  if (codec === 'snes' || codec === 'gba') return 32;
  if (codec === '1bpp') return 8;
  return 16;
}

export function paletteSize(codec: CodecId): number {
  if (codec === 'snes' || codec === 'gba') return 16;
  return 4;
}

export function decodeTile(data: Uint8Array, offset: number, codec: CodecId): Uint8Array | null {
  const size = tileBytes(codec);
  if (offset < 0 || offset + size > data.length) return null;
  const out = new Uint8Array(64);
  if (codec === 'nes') {
    for (let y = 0; y < 8; y += 1) {
      const p0 = data[offset + y];
      const p1 = data[offset + y + 8];
      for (let x = 0; x < 8; x += 1) {
        const bit = 7 - x;
        out[y * 8 + x] = ((p0 >> bit) & 1) | (((p1 >> bit) & 1) << 1);
      }
    }
    return out;
  }
  if (codec === 'gb') {
    for (let y = 0; y < 8; y += 1) {
      const p0 = data[offset + y * 2];
      const p1 = data[offset + y * 2 + 1];
      for (let x = 0; x < 8; x += 1) {
        const bit = 7 - x;
        out[y * 8 + x] = ((p0 >> bit) & 1) | (((p1 >> bit) & 1) << 1);
      }
    }
    return out;
  }
  if (codec === '1bpp') {
    for (let y = 0; y < 8; y += 1) {
      const p0 = data[offset + y];
      for (let x = 0; x < 8; x += 1) out[y * 8 + x] = (p0 >> (7 - x)) & 1;
    }
    return out;
  }
  if (codec === 'snes') {
    for (let y = 0; y < 8; y += 1) {
      const p0 = data[offset + y * 2];
      const p1 = data[offset + y * 2 + 1];
      const p2 = data[offset + 16 + y * 2];
      const p3 = data[offset + 16 + y * 2 + 1];
      for (let x = 0; x < 8; x += 1) {
        const bit = 7 - x;
        out[y * 8 + x] =
          ((p0 >> bit) & 1) |
          (((p1 >> bit) & 1) << 1) |
          (((p2 >> bit) & 1) << 2) |
          (((p3 >> bit) & 1) << 3);
      }
    }
    return out;
  }
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 4; x += 1) {
      const byte = data[offset + y * 4 + x];
      out[y * 8 + x * 2] = byte & 0x0f;
      out[y * 8 + x * 2 + 1] = (byte >> 4) & 0x0f;
    }
  }
  return out;
}

export function encodeNesTile(indices: ArrayLike<number>): Uint8Array {
  const out = new Uint8Array(16);
  for (let y = 0; y < 8; y += 1) {
    let p0 = 0;
    let p1 = 0;
    for (let x = 0; x < 8; x += 1) {
      const v = indices[y * 8 + x] & 3;
      const bit = 7 - x;
      if (v & 1) p0 |= 1 << bit;
      if (v & 2) p1 |= 1 << bit;
    }
    out[y] = p0;
    out[y + 8] = p1;
  }
  return out;
}

export function encodeGbTile(indices: ArrayLike<number>): Uint8Array {
  const out = new Uint8Array(16);
  for (let y = 0; y < 8; y += 1) {
    let p0 = 0;
    let p1 = 0;
    for (let x = 0; x < 8; x += 1) {
      const v = indices[y * 8 + x] & 3;
      const bit = 7 - x;
      if (v & 1) p0 |= 1 << bit;
      if (v & 2) p1 |= 1 << bit;
    }
    out[y * 2] = p0;
    out[y * 2 + 1] = p1;
  }
  return out;
}

export function parseHexColor(hex: string): [number, number, number] {
  const h = hex.replace('#', '');
  const n = h.length === 3 ? h.split('').map((c) => c + c).join('') : h;
  return [parseInt(n.slice(0, 2), 16), parseInt(n.slice(2, 4), 16), parseInt(n.slice(4, 6), 16)];
}

export function renderTiles(
  indices: Uint8Array,
  tilesX: number,
  tilesY: number,
  palette: string[],
  scale: number,
  clearZero: boolean,
): { rgba: Uint8Array; width: number; height: number } {
  const width = tilesX * 8 * scale;
  const height = tilesY * 8 * scale;
  const rgba = new Uint8Array(width * height * 4);
  const colors = palette.map(parseHexColor);
  for (let ty = 0; ty < tilesY; ty += 1) {
    for (let tx = 0; tx < tilesX; tx += 1) {
      const base = (ty * tilesX + tx) * 64;
      for (let y = 0; y < 8; y += 1) {
        for (let x = 0; x < 8; x += 1) {
          const v = indices[base + y * 8 + x] ?? 0;
          const color = colors[v] ?? colors[colors.length - 1] ?? [0, 0, 0];
          const alpha = clearZero && v === 0 ? 0 : 255;
          for (let sy = 0; sy < scale; sy += 1) {
            for (let sx = 0; sx < scale; sx += 1) {
              const px = (tx * 8 + x) * scale + sx;
              const py = (ty * 8 + y) * scale + sy;
              const o = (py * width + px) * 4;
              rgba[o] = color[0];
              rgba[o + 1] = color[1];
              rgba[o + 2] = color[2];
              rgba[o + 3] = alpha;
            }
          }
        }
      }
    }
  }
  return { rgba, width, height };
}

const WORKSHOP_16 = [
  '#1a1612', '#efe6d6', '#e25b2a', '#e4b15a', '#7d9a62', '#3e5340', '#6a4632', '#3a261c',
  '#8fb8c9', '#315868', '#c46a4a', '#f4ecdf', '#d7c7a4', '#8d8070', '#c4526a', '#2a2824',
];

export const PALETTES: Record<string, string[]> = {
  nes: ['#1a1612', '#efe6d6', '#e25b2a', '#e4b15a'],
  gb: ['#1a1612', '#3e5340', '#7d9a62', '#efe6d6'],
  snes: WORKSHOP_16,
  gba: WORKSHOP_16,
  '1bpp': ['#1a1612', '#efe6d6'],
};
