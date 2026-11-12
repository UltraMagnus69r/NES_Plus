export function decodeDpcmLevels(bytes: Uint8Array): number[] {
  const levels: number[] = [];
  let level = 64;
  for (const b of bytes) {
    for (let bit = 0; bit < 8; bit += 1) {
      if (b & (1 << bit)) level = Math.min(126, level + 2);
      else level = Math.max(0, level - 2);
      levels.push(level);
    }
  }
  return levels;
}

export function decodeDpcm(bytes: Uint8Array): Float32Array {
  const levels = decodeDpcmLevels(bytes);
  const out = new Float32Array(levels.length);
  for (let i = 0; i < levels.length; i += 1) out[i] = (levels[i] - 64) / 64;
  return out;
}

function scoreSignal(samples: Float32Array): number {
  if (samples.length < 32) return 0;
  let sum = 0;
  let cross = 0;
  let prev = samples[0];
  let min = 1;
  let max = -1;
  for (const s of samples) {
    sum += s * s;
    if ((prev <= 0 && s > 0) || (prev >= 0 && s < 0)) cross += 1;
    prev = s;
    if (s < min) min = s;
    if (s > max) max = s;
  }
  const rms = Math.sqrt(sum / samples.length);
  const zc = cross / samples.length;
  if (max - min < 0.05) return 0;
  if (zc < 0.002 || zc > 0.45) return rms * 0.2;
  return rms * (1 - Math.abs(zc - 0.05));
}

export type AudioHit = {
  offset: number;
  encoding: 'dpcm' | 'wave' | 'brr';
  samples: Float32Array;
  rate: number;
  score: number;
  label: string;
};

function top(hits: AudioHit[], count: number): AudioHit[] {
  return hits.sort((a, b) => b.score - a.score).slice(0, count);
}

export function scanDpcm(bytes: Uint8Array, start: number, end: number): AudioHit[] {
  const hits: AudioHit[] = [];
  const last = Math.min(bytes.length, end);
  for (let off = start; off + 256 < last; off += 128) {
    const samples = decodeDpcm(bytes.subarray(off, Math.min(last, off + 512)));
    const score = scoreSignal(samples);
    if (score > 0.08) {
      hits.push({ offset: off, encoding: 'dpcm', samples, rate: 8000, score, label: `DPCM @ $${off.toString(16)}` });
    }
  }
  return top(hits, 6);
}

export function decodeGbWave(bytes: Uint8Array): Float32Array {
  const out = new Float32Array(32);
  for (let i = 0; i < 16 && i < bytes.length; i += 1) {
    const b = bytes[i];
    out[i * 2] = ((b >> 4) - 7.5) / 7.5;
    out[i * 2 + 1] = ((b & 0x0f) - 7.5) / 7.5;
  }
  return out;
}

function repeat(samples: Float32Array, times: number): Float32Array {
  const out = new Float32Array(samples.length * times);
  for (let i = 0; i < times; i += 1) out.set(samples, i * samples.length);
  return out;
}

export function scanGbWaves(bytes: Uint8Array): AudioHit[] {
  const hits: AudioHit[] = [];
  const seen = new Set<string>();
  const last = Math.min(bytes.length, 1024 * 1024);
  for (let off = 0; off + 16 <= last; off += 16) {
    const slice = bytes.subarray(off, off + 16);
    const key = [...slice].join(',');
    if (seen.has(key)) continue;
    const samples = decodeGbWave(slice);
    if (new Set(samples).size < 4) continue;
    const score = scoreSignal(repeat(samples, 8));
    if (score <= 0) continue;
    seen.add(key);
    hits.push({
      offset: off,
      encoding: 'wave',
      samples: repeat(samples, 16),
      rate: 4000,
      score,
      label: `Wave @ $${off.toString(16)}`,
    });
    if (hits.length > 40) break;
  }
  return top(hits, 8);
}

export function decodeBrrSample(
  nibble: number,
  shift: number,
  filter: number,
  p1: number,
  p2: number,
): { history: number; output: number } {
  let n = nibble & 0x0f;
  if (n >= 8) n -= 16;
  let sample = n;
  if (shift <= 12) sample = (sample << shift) >> 1;
  else sample &= ~0x07ff;
  if (filter === 1) sample += (p1 >> 1) + ((-p1) >> 5);
  else if (filter === 2) sample += p1 - p2 + (p2 >> 4) + ((-3 * p1) >> 6);
  else if (filter === 3) sample += p1 - p2 + ((-13 * p1) >> 7) + ((3 * p2) >> 4);
  if (sample > 32767) sample = 32767;
  if (sample < -32768) sample = -32768;
  let output = sample << 1;
  if (output > 32767) output = 32767;
  if (output < -32768) output = -32768;
  return { history: sample, output };
}

export function decodeBrr(bytes: Uint8Array, maxBlocks = 64): Float32Array {
  const out: number[] = [];
  let p1 = 0;
  let p2 = 0;
  const blocks = Math.min(maxBlocks, Math.floor(bytes.length / 9));
  for (let b = 0; b < blocks; b += 1) {
    const header = bytes[b * 9];
    const shift = header >> 4;
    const filter = (header >> 2) & 3;
    const end = header & 1;
    for (let i = 0; i < 8; i += 1) {
      const byte = bytes[b * 9 + 1 + i];
      for (const nibble of [byte >> 4, byte & 0x0f]) {
        const step = decodeBrrSample(nibble, shift, filter, p1, p2);
        p2 = p1;
        p1 = step.history;
        out.push(step.output / 32768);
      }
    }
    if (end) break;
  }
  return Float32Array.from(out);
}

export function scanBrr(bytes: Uint8Array): AudioHit[] {
  const hits: AudioHit[] = [];
  const last = Math.min(bytes.length, 1024 * 1024);
  for (let off = 0; off + 72 < last; off += 36) {
    const header = bytes[off];
    const shift = header >> 4;
    const filter = (header >> 2) & 3;
    if (shift > 12 || filter > 3) continue;
    let blocks = 0;
    let ended = false;
    for (let b = 0; b < 48 && off + (b + 1) * 9 <= bytes.length; b += 1) {
      blocks += 1;
      if (bytes[off + b * 9] & 1) {
        ended = true;
        break;
      }
    }
    if (!ended || blocks < 8) continue;
    const samples = decodeBrr(bytes.subarray(off, off + blocks * 9), blocks);
    const score = scoreSignal(samples);
    if (score < 0.05) continue;
    hits.push({ offset: off, encoding: 'brr', samples, rate: 16000, score, label: `BRR @ $${off.toString(16)}` });
    if (hits.length > 30) break;
  }
  return top(hits, 6);
}

export function wavBytes(samples: Float32Array, rate: number): Uint8Array {
  const dataSize = samples.length * 2;
  const buf = new Uint8Array(44 + dataSize);
  const view = new DataView(buf.buffer);
  const write = (o: number, s: string) => {
    for (let i = 0; i < s.length; i += 1) buf[o + i] = s.charCodeAt(i);
  };
  write(0, 'RIFF');
  view.setUint32(4, 36 + dataSize, true);
  write(8, 'WAVE');
  write(12, 'fmt ');
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, 1, true);
  view.setUint32(24, rate, true);
  view.setUint32(28, rate * 2, true);
  view.setUint16(32, 2, true);
  view.setUint16(34, 16, true);
  write(36, 'data');
  view.setUint32(40, dataSize, true);
  for (let i = 0; i < samples.length; i += 1) {
    const s = Math.max(-1, Math.min(1, samples[i]));
    view.setInt16(44 + i * 2, s < 0 ? s * 0x8000 : s * 0x7fff, true);
  }
  return buf;
}

export function wavDataUrl(samples: Float32Array, rate: number): string {
  const wav = wavBytes(samples, rate);
  let binary = '';
  const block = 0x8000;
  for (let i = 0; i < wav.length; i += block) binary += String.fromCharCode(...wav.subarray(i, i + block));
  return `data:audio/wav;base64,${btoa(binary)}`;
}
