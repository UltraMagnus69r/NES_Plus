import { encodeNesTile } from '../rom/codecs';
import { hexToRgba, pngDataUrl } from '../lib/png';

export type SpriteDef = {
  id: string;
  name: string;
  kind: 'player' | 'enemy' | 'terrain' | 'background' | 'foreground' | 'item' | 'hazard' | 'goal';
  behavior: 'player' | 'patrol' | 'collect' | 'hurt' | 'goal' | 'decoration';
  solid: boolean;
  palette: string[];
  pixels: number[][];
};

function grid(rows: string[], map: Record<string, number>): number[][] {
  return rows.map((row) => [...row].map((ch) => map[ch] ?? 0));
}

export const SPRITES: SpriteDef[] = [
  {
    id: 'pip',
    name: 'Pip',
    kind: 'player',
    behavior: 'player',
    solid: false,
    palette: ['#00000000', '#1a1612', '#e4b15a', '#7d9a62'],
    pixels: grid([
    '......ll........',
    '.....llll.......',
    '.....lkkkl......',
    '.....lkgkl......',
    '.....lkkkl......',
    '......lll.......',
    '.......k........',
    '......ggg.......',
    '.....ggkgg......',
    '....ggkkkg......',
    '....gkkkkkg.....',
    '.....ggggg......',
    '......g..g......',
    '.....gg..gg.....',
    '....ggg..ggg....',
    '................'
    ], { '.': 0, 'k': 1, 'g': 2, 'l': 3 }),
  },
  {
    id: 'mite',
    name: 'Soot mite',
    kind: 'enemy',
    behavior: 'patrol',
    solid: false,
    palette: ['#00000000', '#1a1612', '#e25b2a', '#e4b15a'],
    pixels: grid([
    '................',
    '................',
    '................',
    '......kkkk......',
    '.....kggggk.....',
    '....kggeeggk....',
    '....kgeeeegk....',
    '....kggeeggk....',
    '.....kggggk.....',
    '......kkkk......',
    '......k..k......',
    '.....kk..kk.....',
    '................',
    '................',
    '................',
    '................'
    ], { '.': 0, 'k': 1, 'e': 2, 'g': 3 }),
  },
  {
    id: 'loam',
    name: 'Loam',
    kind: 'terrain',
    behavior: 'decoration',
    solid: true,
    palette: ['#3a261c', '#7d9a62', '#6a4632', '#1a1612'],
    pixels: grid([
    'l.lll..ll.lll.l.',
    '.ll.llll.lll.ll.',
    'ssssssssssssssss',
    'skssssssssssssks',
    'ssssssssssssssss',
    'sskksssssskkssss',
    'ssssssssssssssss',
    'skssssssssssssks',
    'ssssssssssssssss',
    'sssskksssskkssss',
    'ssssssssssssssss',
    'skssssssssssssks',
    'ssssssssssssssss',
    'sskksssssskkssss',
    'ssssssssssssssss',
    'kkkkkkkkkkkkkkkk'
    ], { '.': 0, 'l': 1, 's': 2, 'k': 3 }),
  },
  {
    id: 'soil',
    name: 'Soil',
    kind: 'terrain',
    behavior: 'decoration',
    solid: true,
    palette: ['#3a261c', '#6a4632', '#1a1612', '#4a3024'],
    pixels: grid([
    'ssssssssssssssss',
    'skssssssssssssks',
    'ssssssssssssssss',
    'sssskksssskkssss',
    'ssssssssssssssss',
    'skssssssssssssks',
    'ssssssssssssssss',
    'sskksssssskkssss',
    'ssssssssssssssss',
    'skssssssssssssks',
    'ssssssssssssssss',
    'sssskksssskkssss',
    'ssssssssssssssss',
    'skssssssssssssks',
    'ssssssssssssssss',
    'kkkkkkkkkkkkkkkk'
    ], { '.': 0, 's': 1, 'k': 2, 'd': 3 }),
  },
  {
    id: 'brick',
    name: 'Ember brick',
    kind: 'terrain',
    behavior: 'decoration',
    solid: true,
    palette: ['#3a261c', '#1a1612', '#e25b2a', '#e4b15a'],
    pixels: grid([
    'kkkkkkkkkkkkkkkk',
    'keeeekkeeeekkeek',
    'keeeekkeeeekkeek',
    'keeeekkeeeekkeek',
    'kkkkkkkkkkkkkkkk',
    'eekkeeeekkeeeekk',
    'eekkeeeekkeeeekk',
    'eekkeeeekkeeeekk',
    'kkkkkkkkkkkkkkkk',
    'keeeekkeeeekkeek',
    'keeeekkeeeekkeek',
    'keeeekkeeeekkeek',
    'kkkkkkkkkkkkkkkk',
    'eekkeeeekkeeeekk',
    'eekkeeeekkeeeekk',
    'kkkkkkkkkkkkkkkk'
    ], { '.': 0, 'k': 1, 'e': 2, 'g': 3 }),
  },
  {
    id: 'gem',
    name: 'Leaf gem',
    kind: 'item',
    behavior: 'collect',
    solid: false,
    palette: ['#00000000', '#1a1612', '#7d9a62', '#e4b15a'],
    pixels: grid([
    '................',
    '................',
    '.......l........',
    '......lgl.......',
    '.....lgkgl......',
    '....lgkgkgl.....',
    '....lgkkkk......',
    '.....lgkgk......',
    '......lgl.......',
    '.......l........',
    '................',
    '................',
    '................',
    '................',
    '................',
    '................'
    ], { '.': 0, 'k': 1, 'l': 2, 'g': 3 }),
  },
  {
    id: 'thorn',
    name: 'Thorn',
    kind: 'hazard',
    behavior: 'hurt',
    solid: false,
    palette: ['#00000000', '#1a1612', '#e25b2a', '#7d9a62'],
    pixels: grid([
    '................',
    '................',
    '................',
    '................',
    '.......k........',
    '......kkk.......',
    '.....kkekk......',
    '....kkekeekk....',
    '...kkekekekk....',
    '..kkekkkkekk....',
    '.kkkkkkkkkkkk...',
    '................',
    '................',
    '................',
    '................',
    '................'
    ], { '.': 0, 'k': 1, 'e': 2, 'l': 3 }),
  },
  {
    id: 'gate',
    name: 'Blossom gate',
    kind: 'goal',
    behavior: 'goal',
    solid: false,
    palette: ['#00000000', '#1a1612', '#7d9a62', '#e4b15a'],
    pixels: grid([
    '................',
    '..ll........ll..',
    '.llll......llll.',
    '.lkkkl....lkkkl.',
    '.lkgkl....lkgkl.',
    '.lkkkl....lkkkl.',
    '..lll..gg..lll..',
    '.......gg.......',
    '......gggg......',
    '.....ggkkgg.....',
    '.....gkkkg......',
    '.....ggggg......',
    '......gggg......',
    '......g..g......',
    '.....gg..gg.....',
    '....ggg..ggg....'
    ], { '.': 0, 'k': 1, 'l': 2, 'g': 3 }),
  },
  {
    id: 'cloud',
    name: 'Cloud',
    kind: 'background',
    behavior: 'decoration',
    solid: false,
    palette: ['#00000000', '#efe6d6', '#d7c7a4', '#8d8070'],
    pixels: grid([
    '................',
    '................',
    '......pppp......',
    '....pppppppp....',
    '...ppdppppdpp...',
    '..pppppppppppp..',
    '..pppppppppppp..',
    '...pppppppppp...',
    '....kppppppk....',
    '................',
    '................',
    '................',
    '................',
    '................',
    '................',
    '................'
    ], { '.': 0, 'p': 1, 'd': 2, 'k': 3 }),
  },
  {
    id: 'hill',
    name: 'Hill',
    kind: 'background',
    behavior: 'decoration',
    solid: false,
    palette: ['#00000000', '#3e5340', '#7d9a62', '#e4b15a'],
    pixels: grid([
    '................',
    '................',
    '................',
    '................',
    '................',
    '........ll......',
    '.......llll.....',
    '......lldlll....',
    '.....lllldlll...',
    '....llllddllll..',
    '...llllddddlll..',
    '..lllldddddlll..',
    '.llllllllllllll.',
    'dddddddddddddddd',
    'dddddddddddddddd',
    'dddddddddddddddd'
    ], { '.': 0, 'd': 1, 'l': 2, 'g': 3 }),
  },
  {
    id: 'sprout',
    name: 'Sprout',
    kind: 'foreground',
    behavior: 'decoration',
    solid: false,
    palette: ['#00000000', '#1a1612', '#7d9a62', '#e4b15a'],
    pixels: grid([
    '................',
    '.......l........',
    '......lll.......',
    '.....ll.ll......',
    '....ll...ll.....',
    '.......k........',
    '.......k........',
    '......lk........',
    '.....llk........',
    '......kk........',
    '.....lkk........',
    '....llkk........',
    '.......k........',
    '......kkk.......',
    '.....kkkkk......',
    '................'
    ], { '.': 0, 'k': 1, 'l': 2, 'g': 3 }),
  },
];

export function renderSprite(pixels: number[][], palette: string[], scale = 1): string {
  const h = pixels.length;
  const w = pixels[0].length;
  const rgba = new Uint8Array(w * scale * h * scale * 4);
  for (let y = 0; y < h; y += 1) {
    for (let x = 0; x < w; x += 1) {
      const index = pixels[y][x] ?? 0;
      const [r, g, b, a] = index === 0 ? [0, 0, 0, 0] : hexToRgba(palette[index] ?? '#000000', 255);
      for (let sy = 0; sy < scale; sy += 1) {
        for (let sx = 0; sx < scale; sx += 1) {
          const o = ((y * scale + sy) * w * scale + (x * scale + sx)) * 4;
          rgba[o] = r;
          rgba[o + 1] = g;
          rgba[o + 2] = b;
          rgba[o + 3] = a;
        }
      }
    }
  }
  return pngDataUrl(rgba, w * scale, h * scale);
}

function quad(pixels: number[][], ox: number, oy: number) {
  const out = new Uint8Array(64);
  for (let y = 0; y < 8; y += 1) {
    for (let x = 0; x < 8; x += 1) out[y * 8 + x] = pixels[oy + y][ox + x] & 3;
  }
  return out;
}

/** CHR bank whose 16-wide page shows each workshop sprite in its real shape. */
export function demoChr(): Uint8Array {
  const tiles = Array.from({ length: 512 }, () => new Uint8Array(64));
  let col = 0;
  let row = 0;
  for (const sprite of SPRITES) {
    const tl = quad(sprite.pixels, 0, 0);
    const tr = quad(sprite.pixels, 8, 0);
    const bl = quad(sprite.pixels, 0, 8);
    const br = quad(sprite.pixels, 8, 8);
    tiles[row * 16 + col] = tl;
    tiles[row * 16 + col + 1] = tr;
    tiles[(row + 1) * 16 + col] = bl;
    tiles[(row + 1) * 16 + col + 1] = br;
    col += 2;
    if (col >= 16) {
      col = 0;
      row += 2;
    }
  }
  const out = new Uint8Array(8192);
  tiles.forEach((tile, index) => out.set(encodeNesTile(tile), index * 16));
  return out;
}

export function buildDemoRom(): Uint8Array {
  const prg = new Uint8Array(16384);
  const code = [0x78, 0xd8, 0xa2, 0xff, 0x9a, 0x4c, 0x05, 0x80];
  prg.set(code, 0);
  const tag = 'EMBERLEAF DEMO CARTRIDGE';
  for (let i = 0; i < tag.length; i += 1) prg[0x20 + i] = tag.charCodeAt(i);
  prg[0x3ffa] = 0x05;
  prg[0x3ffb] = 0x80;
  prg[0x3ffc] = 0x00;
  prg[0x3ffd] = 0x80;
  prg[0x3ffe] = 0x05;
  prg[0x3fff] = 0x80;
  const chr = demoChr();
  const header = new Uint8Array(16);
  header.set([0x4e, 0x45, 0x53, 0x1a, 1, 1, 0, 0]);
  const rom = new Uint8Array(16 + prg.length + chr.length);
  rom.set(header, 0);
  rom.set(prg, 16);
  rom.set(chr, 16 + prg.length);
  return rom;
}

