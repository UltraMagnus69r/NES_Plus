export type AssetKind =
  | 'player'
  | 'enemy'
  | 'terrain'
  | 'background'
  | 'foreground'
  | 'item'
  | 'hazard'
  | 'goal'
  | 'sfx'
  | 'music';

export type Behavior = 'none' | 'patrol' | 'collect' | 'hurt' | 'goal' | 'player' | 'decoration';

export type Layer = 'back' | 'main' | 'front';

export type Asset = {
  id: string;
  name: string;
  kind: AssetKind;
  src?: string;
  preset?: string;
  w: number;
  h: number;
  solid?: boolean;
  behavior?: Behavior;
};

export type Cell = {
  x: number;
  y: number;
  assetId: string;
  layer: Layer;
};

export type Level = {
  id: string;
  name: string;
  w: number;
  h: number;
  sky: string;
  musicId?: string;
  player: { x: number; y: number };
  cells: Cell[];
};

export type OverNode = {
  id: string;
  name: string;
  levelId: string;
  x: number;
  y: number;
  links: string[];
};

export type Project = {
  version: 1;
  name: string;
  assets: Asset[];
  levels: Level[];
  nodes: OverNode[];
  startNodeId: string;
  playerId?: string;
};

export type Step = 'source' | 'extract' | 'library' | 'levels' | 'overworld' | 'play' | 'export' | 'guide';

export type CodecId = 'nes' | 'gb' | 'snes' | 'gba' | '1bpp';

export type RomFormat = 'nes' | 'gb' | 'gbc' | 'snes' | 'gba' | 'unknown';

export type RomRegion = { label: string; offset: number };

export type RomInfo = {
  format: RomFormat;
  title: string;
  codec: CodecId;
  tileOffset: number;
  details: { label: string; value: string }[];
  warnings: string[];
  regions: RomRegion[];
};

export type RomSession = {
  name: string;
  bytes: Uint8Array;
  info: RomInfo;
};

export const KIND_LAYER: Record<AssetKind, Layer> = {
  player: 'main',
  enemy: 'main',
  terrain: 'main',
  background: 'back',
  foreground: 'front',
  item: 'main',
  hazard: 'main',
  goal: 'main',
  sfx: 'main',
  music: 'main',
};

export const ASSET_KINDS: AssetKind[] = [
  'player',
  'enemy',
  'terrain',
  'background',
  'foreground',
  'item',
  'hazard',
  'goal',
  'sfx',
  'music',
];

export const VISUAL_KINDS: AssetKind[] = [
  'player',
  'enemy',
  'terrain',
  'background',
  'foreground',
  'item',
  'hazard',
  'goal',
];

export function defaultBehavior(kind: AssetKind): Behavior {
  if (kind === 'player') return 'player';
  if (kind === 'enemy') return 'patrol';
  if (kind === 'item') return 'collect';
  if (kind === 'hazard') return 'hurt';
  if (kind === 'goal') return 'goal';
  return 'decoration';
}

export function defaultSolid(kind: AssetKind): boolean {
  return kind === 'terrain';
}
