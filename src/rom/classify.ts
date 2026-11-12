import type { CodecId } from '../types';
import { decodeTile, tileBytes } from './codecs';

export type Suggestion = 'empty' | 'sprite' | 'scenery' | 'text' | 'noise';

export function suggestKind(indices: ArrayLike<number>, w = 8, h = 8): Suggestion {
  const n = w * h;
  let filled = 0;
  let borderClear = 0;
  let border = 0;
  let changes = 0;
  const used = new Set<number>();
  for (let y = 0; y < h; y += 1) {
    for (let x = 0; x < w; x += 1) {
      const v = indices[y * w + x] ?? 0;
      if (v !== 0) filled += 1;
      used.add(v);
      const edge = x === 0 || y === 0 || x === w - 1 || y === h - 1;
      if (edge) {
        border += 1;
        if (v === 0) borderClear += 1;
      }
      if (x > 0 && indices[y * w + x - 1] !== v) changes += 1;
    }
  }
  const fill = filled / n;
  if (fill < 0.04) return 'empty';
  const clearRatio = border === 0 ? 0 : borderClear / border;
  const churn = changes / n;
  if (churn > 0.72 && used.size > 6) return 'noise';
  if (used.size <= 2 && fill > 0.25 && fill < 0.85 && churn > 0.18) return 'text';
  if (clearRatio > 0.55 && fill < 0.72 && fill > 0.08) return 'sprite';
  return 'scenery';
}

export type Candidate = {
  index: number;
  offset: number;
  kind: Suggestion;
  count: number;
};

export function scanCandidates(bytes: Uint8Array, origin: number, codec: CodecId, maxTiles = 6144): Candidate[] {
  const stride = tileBytes(codec);
  const start = Math.max(0, origin - (origin % stride));
  const groups = new Map<string, Candidate>();
  const last = Math.min(bytes.length, start + maxTiles * stride);
  for (let off = start; off + stride <= last; off += stride) {
    const tile = decodeTile(bytes, off, codec);
    if (!tile) continue;
    const kind = suggestKind(tile);
    if (kind === 'empty') continue;
    let key = '';
    for (let i = 0; i < tile.length; i += 1) key += String.fromCharCode(32 + tile[i]);
    const prev = groups.get(key);
    if (prev) prev.count += 1;
    else groups.set(key, { index: Math.floor(off / stride), offset: off, kind, count: 1 });
  }
  const rank: Record<Suggestion, number> = { sprite: 0, scenery: 1, text: 2, noise: 3, empty: 4 };
  return [...groups.values()].sort((a, b) => rank[a.kind] - rank[b.kind] || b.count - a.count).slice(0, 180);
}
