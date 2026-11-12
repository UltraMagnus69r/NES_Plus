import type { CodecId, RomFormat, RomInfo, RomRegion } from '../types';

export type InesLayout = {
  nes2: boolean;
  prgBytes: number;
  chrBytes: number;
  trainerBytes: number;
  prgStart: number;
  chrStart: number;
  prg: Uint8Array;
  chr: Uint8Array;
  trainer: Uint8Array | null;
  chrRam: boolean;
  mapper: number;
  mirroring: 'horizontal' | 'vertical' | 'four';
};

function nes2Size(bankByte: number, nibble: number, unit: number): number {
  if (nibble !== 0x0f) return ((nibble << 8) | bankByte) * unit;
  const exponent = (bankByte & 0xfc) >> 2;
  const multiplier = bankByte & 0x03;
  return 2 ** exponent * (multiplier * 2 + 1);
}

function ascii(bytes: Uint8Array, offset: number, length: number): string {
  let out = '';
  for (let i = 0; i < length && offset + i < bytes.length; i += 1) {
    const c = bytes[offset + i];
    out += c >= 32 && c < 127 ? String.fromCharCode(c) : ' ';
  }
  return out.replace(/\0/g, '').trim();
}

function read16(bytes: Uint8Array, offset: number): number {
  if (offset + 1 >= bytes.length) return 0;
  return bytes[offset] | (bytes[offset + 1] << 8);
}

function slice(bytes: Uint8Array, start: number, length: number): Uint8Array {
  if (length <= 0 || start >= bytes.length) return new Uint8Array();
  return bytes.subarray(start, Math.min(bytes.length, start + length));
}

export function parseInes(bytes: Uint8Array): InesLayout | null {
  if (bytes.length < 16) return null;
  if (bytes[0] !== 0x4e || bytes[1] !== 0x45 || bytes[2] !== 0x53 || bytes[3] !== 0x1a) return null;
  const flags6 = bytes[6];
  const flags7 = bytes[7];
  const nes2 = (flags7 & 0x0c) === 0x08;
  let prgBytes = bytes[4] * 16384;
  let chrBytes = bytes[5] * 8192;
  if (nes2 && bytes.length >= 10) {
    prgBytes = nes2Size(bytes[4], bytes[9] & 0x0f, 16384);
    chrBytes = nes2Size(bytes[5], (bytes[9] >> 4) & 0x0f, 8192);
  }
  const trainerBytes = flags6 & 0x04 ? 512 : 0;
  const prgStart = 16 + trainerBytes;
  const chrStart = prgStart + prgBytes;
  const four = (flags6 & 0x08) !== 0;
  const vertical = (flags6 & 0x01) !== 0;
  return {
    nes2,
    prgBytes,
    chrBytes,
    trainerBytes,
    prgStart,
    chrStart,
    prg: slice(bytes, prgStart, prgBytes),
    chr: slice(bytes, chrStart, chrBytes),
    trainer: trainerBytes ? slice(bytes, 16, trainerBytes) : null,
    chrRam: chrBytes === 0,
    mapper: (flags6 >> 4) | (flags7 & 0xf0),
    mirroring: four ? 'four' : vertical ? 'vertical' : 'horizontal',
  };
}

function gbChecksum(bytes: Uint8Array): number {
  let x = 0;
  for (let i = 0x134; i <= 0x14c; i += 1) x = (x - bytes[i] - 1) & 0xff;
  return x;
}

function gbaChecksum(bytes: Uint8Array): number {
  let sum = 0;
  for (let i = 0xa0; i <= 0xbc; i += 1) sum = (sum - bytes[i]) & 0xff;
  return (sum - 0x19) & 0xff;
}

function snesScore(bytes: Uint8Array, base: number): number {
  if (base < 0 || base + 0x20 > bytes.length) return -1;
  let printable = 0;
  for (let i = 0; i < 21; i += 1) {
    const c = bytes[base + i];
    if ((c >= 32 && c < 127) || c === 0) printable += 1;
  }
  const complement = read16(bytes, base + 0x1c);
  const checksum = read16(bytes, base + 0x1e);
  let score = printable;
  if ((complement ^ checksum) === 0xffff && (checksum !== 0 || complement !== 0xffff)) score += 80;
  const map = bytes[base + 0x15];
  if (map === 0x20 || map === 0x21 || map === 0x30 || map === 0x31 || map === 0x32 || map === 0x23) score += 12;
  const romSize = bytes[base + 0x17];
  if (romSize >= 0x08 && romSize <= 0x0d) score += 4;
  return score;
}

export function detectRom(bytes: Uint8Array, filename: string): RomInfo {
  const warnings: string[] = [];
  const details: { label: string; value: string }[] = [
    { label: 'File', value: filename },
    { label: 'Size', value: `${bytes.length.toLocaleString()} bytes` },
  ];
  const regions: RomRegion[] = [{ label: 'Start', offset: 0 }];

  const ines = parseInes(bytes);
  if (ines) {
    if (ines.chrRam) {
      warnings.push(
        'CHR size is zero, so this cartridge uses CHR RAM. Pattern tables are filled by the program and are not sitting in the file as a sheet. You can still page PRG bytes as NES tiles. Banked or compressed graphics need a viewer such as Mesen.',
      );
    }
    details.push(
      { label: 'Format', value: ines.nes2 ? 'NES 2.0' : 'iNES' },
      { label: 'PRG', value: `${ines.prgBytes / 1024} KB` },
      { label: 'CHR', value: ines.chrRam ? 'CHR RAM (no CHR in file)' : `${ines.chrBytes / 1024} KB` },
      { label: 'Mapper', value: String(ines.mapper) },
      { label: 'Trainer', value: ines.trainerBytes ? `${ines.trainerBytes} bytes` : 'none' },
      { label: 'Mirroring', value: ines.mirroring },
    );
    if (ines.trainerBytes) regions.push({ label: 'Trainer', offset: 16 });
    regions.push({ label: 'PRG', offset: ines.prgStart });
    if (!ines.chrRam) regions.push({ label: 'CHR', offset: Math.min(ines.chrStart, Math.max(0, bytes.length - 1)) });
    return {
      format: 'nes',
      title: filename.replace(/\.[^.]+$/, ''),
      codec: 'nes',
      tileOffset: ines.chrRam ? ines.prgStart : Math.min(ines.chrStart, bytes.length),
      details,
      warnings,
      regions,
    };
  }

  if (bytes.length > 0x150) {
    const sum = gbChecksum(bytes);
    const title = ascii(bytes, 0x134, 16);
    const letters = (title.match(/[A-Za-z0-9]/g) ?? []).length;
    if (sum === bytes[0x14d] && letters >= 3) {
      const cgb = bytes[0x143];
      const format: RomFormat = cgb === 0x80 || cgb === 0xc0 ? 'gbc' : 'gb';
      const shown = ascii(bytes, 0x134, format === 'gbc' ? 15 : 16) || filename;
      details.push(
        { label: 'Format', value: format === 'gbc' ? 'Game Boy Color' : 'Game Boy' },
        { label: 'Title', value: shown },
        { label: 'Header checksum', value: 'matches' },
        { label: 'Cartridge type', value: `$${bytes[0x147].toString(16).padStart(2, '0')}` },
        { label: 'ROM size code', value: `$${bytes[0x148].toString(16).padStart(2, '0')}` },
      );
      warnings.push(
        'Tiles are uncompressed 2bpp and are usually scattered, not labeled. Many games copy them into VRAM from compressed banks this viewer does not unpack.',
      );
      regions.push({ label: 'Header', offset: 0x100 }, { label: 'ROM', offset: 0x150 });
      return { format, title: shown, codec: 'gb', tileOffset: 0, details, warnings, regions };
    }
  }

  if (bytes.length > 0xc0 && bytes[0xbc] === 0x96 && gbaChecksum(bytes) === bytes[0xbd]) {
    const title = ascii(bytes, 0xa0, 12) || filename;
    const code = ascii(bytes, 0xac, 4);
    details.push(
      { label: 'Format', value: 'Game Boy Advance' },
      { label: 'Title', value: title },
      { label: 'Code', value: code || '—' },
      { label: 'Header checksum', value: 'matches' },
    );
    warnings.push(
      'GBA graphics are often compressed. This viewer pages uncompressed 4bpp tiles. It does not decrypt anything. Switch, 3DS, and Wii U images are not supported.',
    );
    regions.push({ label: 'Header', offset: 0 }, { label: 'Entry', offset: 0xc0 });
    return { format: 'gba', title, codec: 'gba', tileOffset: 0, details, warnings, regions };
  }

  let smc = 0;
  if (bytes.length > 512 && bytes.length % 1024 === 512) {
    smc = 512;
    warnings.push('File length mod 1024 is 512, so a 512-byte SMC copier header is likely. Header scores include that shift.');
  }
  const candidates = [
    { layout: 'LoROM', base: smc + 0x7fc0, score: snesScore(bytes, smc + 0x7fc0) },
    { layout: 'HiROM', base: smc + 0xffc0, score: snesScore(bytes, smc + 0xffc0) },
  ];
  if (smc === 512) {
    candidates.push(
      { layout: 'LoROM, no copier header', base: 0x7fc0, score: snesScore(bytes, 0x7fc0) },
      { layout: 'HiROM, no copier header', base: 0xffc0, score: snesScore(bytes, 0xffc0) },
    );
  }
  const best = candidates.reduce((a, b) => (b.score > a.score ? b : a));
  if (best.score >= 70) {
    const title = ascii(bytes, best.base, 21) || filename;
    const complement = read16(bytes, best.base + 0x1c);
    const checksum = read16(bytes, best.base + 0x1e);
    details.push(
      { label: 'Format', value: `Super NES ${best.layout}` },
      { label: 'Title', value: title },
      { label: 'Map mode', value: `$${(bytes[best.base + 0x15] ?? 0).toString(16).padStart(2, '0')}` },
      { label: 'Checksum', value: (complement ^ checksum) === 0xffff ? 'complement matches' : 'complement does not match' },
      { label: 'Copier header', value: smc === 512 ? '512 bytes' : 'none' },
    );
    warnings.push(
      'Uncompressed 4bpp tiles page here. A lot of Super NES art is compressed. RetroTool and Mesen go further on codecs, scripts, and live VRAM.',
    );
    regions.push({ label: 'Header', offset: best.base });
    if (smc) regions.push({ label: 'After copier header', offset: smc });
    return { format: 'snes', title, codec: 'snes', tileOffset: smc, details, warnings, regions };
  }

  warnings.push(
    'No iNES, Game Boy, Super NES, or Game Boy Advance header matched. The bytes still open in the tile viewer. Encrypted Switch, 3DS, and Wii U dumps are not supported.',
  );
  const ext = filename.split('.').pop()?.toLowerCase() ?? '';
  const codec: CodecId =
    ext === 'gb' || ext === 'gbc' ? 'gb' : ext === 'gba' ? 'gba' : ext === 'sfc' || ext === 'smc' ? 'snes' : 'nes';
  return { format: 'unknown', title: filename, codec, tileOffset: 0, details, warnings, regions };
}

export function findStrings(bytes: Uint8Array, limit = 200): { offset: number; text: string }[] {
  const found: { offset: number; text: string }[] = [];
  let start = -1;
  let text = '';
  const flush = () => {
    if (text.length >= 6) found.push({ offset: start, text });
    start = -1;
    text = '';
  };
  const scan = Math.min(bytes.length, 2 * 1024 * 1024);
  for (let i = 0; i < scan; i += 1) {
    const c = bytes[i];
    if (c >= 32 && c < 127) {
      if (start < 0) start = i;
      text += String.fromCharCode(c);
      if (text.length > 96) flush();
    } else if (start >= 0) flush();
    if (found.length >= limit) break;
  }
  if (start >= 0) flush();
  return found.slice(0, limit);
}
