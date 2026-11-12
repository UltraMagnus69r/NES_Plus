import { SPRITES, buildDemoRom, renderSprite } from '../art/sprites';
import type { Asset, Cell, Level, Project } from '../types';
import blocklist from '../../shared/blocklist.json';

const KEY = 'emberleaf.project.v1';

export function blockedName(name: string): string | null {
  const hay = name.toLowerCase();
  const hit = (blocklist as string[]).find((word) => hay.includes(word));
  return hit ? `Emberleaf will not label a workshop part "${hit}". Name the original part you are making.` : null;
}

function stamp(cells: Cell[], id: string, layer: Cell['layer'], x0: number, y0: number, x1: number, y1: number) {
  for (let y = y0; y <= y1; y += 1) {
    for (let x = x0; x <= x1; x += 1) cells.push({ x, y, assetId: id, layer });
  }
}

function courseOne(): Level {
  const cells: Cell[] = [];
  stamp(cells, 'loam', 'main', 0, 11, 13, 11);
  stamp(cells, 'loam', 'main', 16, 11, 31, 11);
  stamp(cells, 'soil', 'main', 0, 12, 13, 13);
  stamp(cells, 'soil', 'main', 16, 12, 31, 13);
  stamp(cells, 'brick', 'main', 22, 9, 26, 9);
  stamp(cells, 'cloud', 'back', 3, 2, 3, 2);
  stamp(cells, 'cloud', 'back', 14, 3, 14, 3);
  stamp(cells, 'cloud', 'back', 24, 2, 24, 2);
  stamp(cells, 'hill', 'back', 6, 6, 6, 6);
  stamp(cells, 'hill', 'back', 20, 6, 20, 6);
  stamp(cells, 'sprout', 'front', 4, 10, 4, 10);
  stamp(cells, 'sprout', 'front', 18, 10, 18, 10);
  cells.push({ x: 8, y: 10, assetId: 'gem', layer: 'main' });
  cells.push({ x: 20, y: 10, assetId: 'mite', layer: 'main' });
  cells.push({ x: 29, y: 10, assetId: 'gate', layer: 'main' });
  return {
    id: 'level-seed',
    name: 'Seed Trail',
    w: 32,
    h: 14,
    sky: '#8fb8c9',
    musicId: 'music-grove',
    player: { x: 2, y: 10 },
    cells,
  };
}

function courseTwo(): Level {
  const cells: Cell[] = [];
  stamp(cells, 'loam', 'main', 0, 11, 14, 11);
  stamp(cells, 'loam', 'main', 18, 11, 35, 11);
  stamp(cells, 'soil', 'main', 0, 12, 14, 13);
  stamp(cells, 'soil', 'main', 18, 12, 35, 13);
  stamp(cells, 'brick', 'main', 22, 8, 27, 8);
  cells.push({ x: 16, y: 12, assetId: 'thorn', layer: 'main' });
  cells.push({ x: 17, y: 12, assetId: 'thorn', layer: 'main' });
  cells.push({ x: 10, y: 10, assetId: 'mite', layer: 'main' });
  cells.push({ x: 24, y: 7, assetId: 'mite', layer: 'main' });
  cells.push({ x: 12, y: 9, assetId: 'gem', layer: 'main' });
  cells.push({ x: 33, y: 10, assetId: 'gate', layer: 'main' });
  stamp(cells, 'cloud', 'back', 4, 2, 4, 2);
  stamp(cells, 'hill', 'back', 8, 6, 8, 6);
  stamp(cells, 'sprout', 'front', 3, 10, 3, 10);
  stamp(cells, 'sprout', 'front', 20, 10, 20, 10);
  return {
    id: 'level-cinder',
    name: 'Cinder Stair',
    w: 36,
    h: 14,
    sky: '#c46a4a',
    musicId: 'music-grove',
    player: { x: 2, y: 10 },
    cells,
  };
}

export function createDefaultProject(): Project {
  const assets: Asset[] = SPRITES.map((sprite) => ({
    id: sprite.id,
    name: sprite.name,
    kind: sprite.kind,
    behavior: sprite.behavior,
    solid: sprite.solid,
    src: renderSprite(sprite.pixels, sprite.palette, 1),
    w: 16,
    h: 16,
  }));
  assets.push(
    { id: 'sfx-jump', name: 'Jump pip', kind: 'sfx', preset: 'jump', w: 0, h: 0, behavior: 'none' },
    { id: 'sfx-stomp', name: 'Stomp', kind: 'sfx', preset: 'stomp', w: 0, h: 0, behavior: 'none' },
    { id: 'sfx-collect', name: 'Gem', kind: 'sfx', preset: 'collect', w: 0, h: 0, behavior: 'none' },
    { id: 'sfx-hurt', name: 'Hurt', kind: 'sfx', preset: 'hurt', w: 0, h: 0, behavior: 'none' },
    { id: 'sfx-goal', name: 'Gate', kind: 'sfx', preset: 'goal', w: 0, h: 0, behavior: 'none' },
    { id: 'music-grove', name: 'Seed grove', kind: 'music', preset: 'theme', w: 0, h: 0, behavior: 'none' },
  );
  return {
    version: 1,
    name: 'Emberleaf',
    assets,
    levels: [courseOne(), courseTwo()],
    nodes: [
      { id: 'node-seed', name: 'Seed Trail', levelId: 'level-seed', x: 78, y: 110, links: ['node-cinder'] },
      { id: 'node-cinder', name: 'Cinder Stair', levelId: 'level-cinder', x: 230, y: 72, links: ['node-seed'] },
    ],
    startNodeId: 'node-seed',
    playerId: 'pip',
  };
}

export function buildDemoCartridge(): Uint8Array {
  return buildDemoRom();
}

function validProject(value: unknown): value is Project {
  if (!value || typeof value !== 'object') return false;
  const project = value as Project;
  return project.version === 1 && Array.isArray(project.assets) && Array.isArray(project.levels) && Array.isArray(project.nodes);
}

export function loadProject(): Project {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return createDefaultProject();
    const parsed = JSON.parse(raw) as unknown;
    if (!validProject(parsed)) return createDefaultProject();
    return parsed;
  } catch {
    return createDefaultProject();
  }
}

export function saveProject(project: Project): 'saved' | 'error' {
  try {
    const copy = { ...project };
    localStorage.setItem(KEY, JSON.stringify(copy));
    return 'saved';
  } catch {
    return 'error';
  }
}

export function projectJson(project: Project): string {
  return JSON.stringify(project, null, 2);
}
