type Mode = 'imp' | 'imm' | 'zp' | 'zpx' | 'zpy' | 'abs' | 'abx' | 'aby' | 'ind' | 'izx' | 'izy' | 'rel' | 'acc';

const OPS: Record<number, [string, Mode]> = {
  0x00: ['BRK', 'imp'], 0x01: ['ORA', 'izx'], 0x05: ['ORA', 'zp'], 0x06: ['ASL', 'zp'],
  0x08: ['PHP', 'imp'], 0x09: ['ORA', 'imm'], 0x0a: ['ASL', 'acc'], 0x0d: ['ORA', 'abs'], 0x0e: ['ASL', 'abs'],
  0x10: ['BPL', 'rel'], 0x11: ['ORA', 'izy'], 0x15: ['ORA', 'zpx'], 0x16: ['ASL', 'zpx'],
  0x18: ['CLC', 'imp'], 0x19: ['ORA', 'aby'], 0x1d: ['ORA', 'abx'], 0x1e: ['ASL', 'abx'],
  0x20: ['JSR', 'abs'], 0x21: ['AND', 'izx'], 0x24: ['BIT', 'zp'], 0x25: ['AND', 'zp'], 0x26: ['ROL', 'zp'],
  0x28: ['PLP', 'imp'], 0x29: ['AND', 'imm'], 0x2a: ['ROL', 'acc'], 0x2c: ['BIT', 'abs'], 0x2d: ['AND', 'abs'], 0x2e: ['ROL', 'abs'],
  0x30: ['BMI', 'rel'], 0x31: ['AND', 'izy'], 0x35: ['AND', 'zpx'], 0x36: ['ROL', 'zpx'],
  0x38: ['SEC', 'imp'], 0x39: ['AND', 'aby'], 0x3d: ['AND', 'abx'], 0x3e: ['ROL', 'abx'],
  0x40: ['RTI', 'imp'], 0x41: ['EOR', 'izx'], 0x45: ['EOR', 'zp'], 0x46: ['LSR', 'zp'],
  0x48: ['PHA', 'imp'], 0x49: ['EOR', 'imm'], 0x4a: ['LSR', 'acc'], 0x4c: ['JMP', 'abs'], 0x4d: ['EOR', 'abs'], 0x4e: ['LSR', 'abs'],
  0x50: ['BVC', 'rel'], 0x51: ['EOR', 'izy'], 0x55: ['EOR', 'zpx'], 0x56: ['LSR', 'zpx'],
  0x58: ['CLI', 'imp'], 0x59: ['EOR', 'aby'], 0x5d: ['EOR', 'abx'], 0x5e: ['LSR', 'abx'],
  0x60: ['RTS', 'imp'], 0x61: ['ADC', 'izx'], 0x65: ['ADC', 'zp'], 0x66: ['ROR', 'zp'],
  0x68: ['PLA', 'imp'], 0x69: ['ADC', 'imm'], 0x6a: ['ROR', 'acc'], 0x6c: ['JMP', 'ind'], 0x6d: ['ADC', 'abs'], 0x6e: ['ROR', 'abs'],
  0x70: ['BVS', 'rel'], 0x71: ['ADC', 'izy'], 0x75: ['ADC', 'zpx'], 0x76: ['ROR', 'zpx'],
  0x78: ['SEI', 'imp'], 0x79: ['ADC', 'aby'], 0x7d: ['ADC', 'abx'], 0x7e: ['ROR', 'abx'],
  0x81: ['STA', 'izx'], 0x84: ['STY', 'zp'], 0x85: ['STA', 'zp'], 0x86: ['STX', 'zp'],
  0x88: ['DEY', 'imp'], 0x8a: ['TXA', 'imp'], 0x8c: ['STY', 'abs'], 0x8d: ['STA', 'abs'], 0x8e: ['STX', 'abs'],
  0x90: ['BCC', 'rel'], 0x91: ['STA', 'izy'], 0x94: ['STY', 'zpx'], 0x95: ['STA', 'zpx'], 0x96: ['STX', 'zpy'],
  0x98: ['TYA', 'imp'], 0x99: ['STA', 'aby'], 0x9a: ['TXS', 'imp'], 0x9d: ['STA', 'abx'],
  0xa0: ['LDY', 'imm'], 0xa1: ['LDA', 'izx'], 0xa2: ['LDX', 'imm'], 0xa4: ['LDY', 'zp'], 0xa5: ['LDA', 'zp'], 0xa6: ['LDX', 'zp'],
  0xa8: ['TAY', 'imp'], 0xa9: ['LDA', 'imm'], 0xaa: ['TAX', 'imp'], 0xac: ['LDY', 'abs'], 0xad: ['LDA', 'abs'], 0xae: ['LDX', 'abs'],
  0xb0: ['BCS', 'rel'], 0xb1: ['LDA', 'izy'], 0xb4: ['LDY', 'zpx'], 0xb5: ['LDA', 'zpx'], 0xb6: ['LDX', 'zpy'],
  0xb8: ['CLV', 'imp'], 0xb9: ['LDA', 'aby'], 0xba: ['TSX', 'imp'], 0xbc: ['LDY', 'abx'], 0xbd: ['LDA', 'abx'], 0xbe: ['LDX', 'aby'],
  0xc0: ['CPY', 'imm'], 0xc1: ['CMP', 'izx'], 0xc4: ['CPY', 'zp'], 0xc5: ['CMP', 'zp'], 0xc6: ['DEC', 'zp'],
  0xc8: ['INY', 'imp'], 0xc9: ['CMP', 'imm'], 0xca: ['DEX', 'imp'], 0xcc: ['CPY', 'abs'], 0xcd: ['CMP', 'abs'], 0xce: ['DEC', 'abs'],
  0xd0: ['BNE', 'rel'], 0xd1: ['CMP', 'izy'], 0xd5: ['CMP', 'zpx'], 0xd6: ['DEC', 'zpx'],
  0xd8: ['CLD', 'imp'], 0xd9: ['CMP', 'aby'], 0xdd: ['CMP', 'abx'], 0xde: ['DEC', 'abx'],
  0xe0: ['CPX', 'imm'], 0xe1: ['SBC', 'izx'], 0xe4: ['CPX', 'zp'], 0xe5: ['SBC', 'zp'], 0xe6: ['INC', 'zp'],
  0xe8: ['INX', 'imp'], 0xe9: ['SBC', 'imm'], 0xea: ['NOP', 'imp'], 0xec: ['CPX', 'abs'], 0xed: ['SBC', 'abs'], 0xee: ['INC', 'abs'],
  0xf0: ['BEQ', 'rel'], 0xf1: ['SBC', 'izy'], 0xf5: ['SBC', 'zpx'], 0xf6: ['INC', 'zpx'],
  0xf8: ['SED', 'imp'], 0xf9: ['SBC', 'aby'], 0xfd: ['SBC', 'abx'], 0xfe: ['INC', 'abx'],
};

const LEN: Record<Mode, number> = {
  imp: 1, acc: 1, imm: 2, zp: 2, zpx: 2, zpy: 2, rel: 2,
  abs: 3, abx: 3, aby: 3, ind: 3, izx: 2, izy: 2,
};

function hex(n: number, w = 2): string {
  return (n >>> 0).toString(16).toUpperCase().padStart(w, '0');
}

function operand(mode: Mode, bytes: Uint8Array, pc: number): string {
  const b1 = bytes[pc + 1] ?? 0;
  const b2 = bytes[pc + 2] ?? 0;
  const abs = b1 | (b2 << 8);
  switch (mode) {
    case 'imm': return `#$${hex(b1)}`;
    case 'zp': return `$${hex(b1)}`;
    case 'zpx': return `$${hex(b1)},X`;
    case 'zpy': return `$${hex(b1)},Y`;
    case 'abs': return `$${hex(abs, 4)}`;
    case 'abx': return `$${hex(abs, 4)},X`;
    case 'aby': return `$${hex(abs, 4)},Y`;
    case 'ind': return `($${hex(abs, 4)})`;
    case 'izx': return `($${hex(b1)},X)`;
    case 'izy': return `($${hex(b1)}),Y`;
    case 'rel': {
      const rel = b1 < 128 ? b1 : b1 - 256;
      return `$${hex((pc + 2 + rel) & 0xffff, 4)}`;
    }
    case 'acc': return 'A';
    default: return '';
  }
}

export type AsmLine = { address: number; bytes: string; text: string };

export function disassemble6502(bytes: Uint8Array, address: number, count = 48): AsmLine[] {
  const lines: AsmLine[] = [];
  let pc = Math.max(0, address);
  for (let n = 0; n < count && pc < bytes.length; n += 1) {
    const op = bytes[pc];
    const known = OPS[op];
    if (!known) {
      lines.push({ address: pc, bytes: hex(op), text: `.db $${hex(op)}` });
      pc += 1;
      continue;
    }
    const [name, mode] = known;
    const len = LEN[mode];
    const raw = [...bytes.subarray(pc, Math.min(bytes.length, pc + len))].map((b) => hex(b)).join(' ');
    const arg = operand(mode, bytes, pc);
    lines.push({ address: pc, bytes: raw, text: arg ? `${name} ${arg}` : name });
    pc += len;
  }
  return lines;
}

const R8 = ['B', 'C', 'D', 'E', 'H', 'L', '(HL)', 'A'] as const;
const RP = ['BC', 'DE', 'HL', 'SP'] as const;
const RP2 = ['BC', 'DE', 'HL', 'AF'] as const;
const CC = ['NZ', 'Z', 'NC', 'C'] as const;

function u8(bytes: Uint8Array, pc: number): number {
  return bytes[pc] ?? 0;
}
function u16(bytes: Uint8Array, pc: number): number {
  return u8(bytes, pc) | (u8(bytes, pc + 1) << 8);
}
function relTarget(pc: number, disp: number): string {
  const rel = disp < 128 ? disp : disp - 256;
  return `$${hex((pc + 2 + rel) & 0xffff, 4)}`;
}

export function disassembleLr35902(bytes: Uint8Array, address: number, count = 48): AsmLine[] {
  const lines: AsmLine[] = [];
  let pc = Math.max(0, address);
  for (let n = 0; n < count && pc < bytes.length; n += 1) {
    const op = bytes[pc];
    if (op === 0xcb) {
      if (pc + 1 >= bytes.length) {
        lines.push({ address: pc, bytes: hex(op), text: '.db $CB' });
        break;
      }
      const cb = bytes[pc + 1];
      const reg = R8[cb & 7];
      const bit = (cb >> 3) & 7;
      let text = `SET ${bit}, ${reg}`;
      if (cb < 0x08) text = `RLC ${reg}`;
      else if (cb < 0x10) text = `RRC ${reg}`;
      else if (cb < 0x18) text = `RL ${reg}`;
      else if (cb < 0x20) text = `RR ${reg}`;
      else if (cb < 0x28) text = `SLA ${reg}`;
      else if (cb < 0x30) text = `SRA ${reg}`;
      else if (cb < 0x38) text = `SWAP ${reg}`;
      else if (cb < 0x40) text = `SRL ${reg}`;
      else if (cb < 0x80) text = `BIT ${bit}, ${reg}`;
      else if (cb < 0xc0) text = `RES ${bit}, ${reg}`;
      lines.push({ address: pc, bytes: `${hex(op)} ${hex(cb)}`, text });
      pc += 2;
      continue;
    }
    const decoded = lrOpcode(op, bytes, pc);
    lines.push({ address: pc, bytes: decoded.bytes, text: decoded.text });
    pc += decoded.len;
  }
  return lines;
}

function lrOpcode(op: number, bytes: Uint8Array, pc: number): { len: number; bytes: string; text: string } {
  const b1 = u8(bytes, pc + 1);
  const word = u16(bytes, pc + 1);
  const raw = (len: number) => [...bytes.subarray(pc, Math.min(bytes.length, pc + len))].map((b) => hex(b)).join(' ');
  const one = (text: string) => ({ len: 1, bytes: raw(1), text });
  const two = (text: string) => ({ len: 2, bytes: raw(2), text });
  const three = (text: string) => ({ len: 3, bytes: raw(3), text });
  if (op === 0x00) return one('NOP');
  if (op === 0x10) return two('STOP');
  if (op === 0x76) return one('HALT');
  if (op === 0x07) return one('RLCA');
  if (op === 0x0f) return one('RRCA');
  if (op === 0x17) return one('RLA');
  if (op === 0x1f) return one('RRA');
  if (op === 0x27) return one('DAA');
  if (op === 0x2f) return one('CPL');
  if (op === 0x37) return one('SCF');
  if (op === 0x3f) return one('CCF');
  if (op === 0xf3) return one('DI');
  if (op === 0xfb) return one('EI');
  if (op === 0xe9) return one('JP (HL)');
  if (op === 0xc9) return one('RET');
  if (op === 0xd9) return one('RETI');
  if (op === 0x18) return two(`JR ${relTarget(pc, b1)}`);
  if (op === 0xc3) return three(`JP $${hex(word, 4)}`);
  if (op === 0xcd) return three(`CALL $${hex(word, 4)}`);
  if (op === 0xc6) return two(`ADD A, $${hex(b1)}`);
  if (op === 0xce) return two(`ADC A, $${hex(b1)}`);
  if (op === 0xd6) return two(`SUB $${hex(b1)}`);
  if (op === 0xde) return two(`SBC A, $${hex(b1)}`);
  if (op === 0xe6) return two(`AND $${hex(b1)}`);
  if (op === 0xee) return two(`XOR $${hex(b1)}`);
  if (op === 0xf6) return two(`OR $${hex(b1)}`);
  if (op === 0xfe) return two(`CP $${hex(b1)}`);
  if (op === 0xe0) return two(`LDH ($${hex(b1)}), A`);
  if (op === 0xf0) return two(`LDH A, ($${hex(b1)})`);
  if (op === 0xe2) return one('LD (C), A');
  if (op === 0xf2) return one('LD A, (C)');
  if (op === 0xea) return three(`LD ($${hex(word, 4)}), A`);
  if (op === 0xfa) return three(`LD A, ($${hex(word, 4)})`);
  if (op === 0x08) return three(`LD ($${hex(word, 4)}), SP`);
  if (op === 0xf8) return two(`LD HL, SP+$${hex(b1)}`);
  if (op === 0xf9) return one('LD SP, HL');
  if (op === 0x22) return one('LD (HL+), A');
  if (op === 0x2a) return one('LD A, (HL+)');
  if (op === 0x32) return one('LD (HL-), A');
  if (op === 0x3a) return one('LD A, (HL-)');
  if (op === 0x02) return one('LD (BC), A');
  if (op === 0x12) return one('LD (DE), A');
  if (op === 0x0a) return one('LD A, (BC)');
  if (op === 0x1a) return one('LD A, (DE)');
  if ((op & 0xcf) === 0x01) return three(`LD ${RP[(op >> 4) & 3]}, $${hex(word, 4)}`);
  if ((op & 0xcf) === 0x09) return one(`ADD HL, ${RP[(op >> 4) & 3]}`);
  if ((op & 0xcf) === 0x03) return one(`INC ${RP[(op >> 4) & 3]}`);
  if ((op & 0xcf) === 0x0b) return one(`DEC ${RP[(op >> 4) & 3]}`);
  if ((op & 0xc7) === 0x04) return one(`INC ${R8[(op >> 3) & 7]}`);
  if ((op & 0xc7) === 0x05) return one(`DEC ${R8[(op >> 3) & 7]}`);
  if ((op & 0xc7) === 0x06) return two(`LD ${R8[(op >> 3) & 7]}, $${hex(b1)}`);
  if (op >= 0x40 && op <= 0x7f) return one(`LD ${R8[(op >> 3) & 7]}, ${R8[op & 7]}`);
  if (op >= 0x80 && op <= 0x87) return one(`ADD A, ${R8[op & 7]}`);
  if (op >= 0x88 && op <= 0x8f) return one(`ADC A, ${R8[op & 7]}`);
  if (op >= 0x90 && op <= 0x97) return one(`SUB ${R8[op & 7]}`);
  if (op >= 0x98 && op <= 0x9f) return one(`SBC A, ${R8[op & 7]}`);
  if (op >= 0xa0 && op <= 0xa7) return one(`AND ${R8[op & 7]}`);
  if (op >= 0xa8 && op <= 0xaf) return one(`XOR ${R8[op & 7]}`);
  if (op >= 0xb0 && op <= 0xb7) return one(`OR ${R8[op & 7]}`);
  if (op >= 0xb8 && op <= 0xbf) return one(`CP ${R8[op & 7]}`);
  if ((op & 0xe7) === 0x20) return two(`JR ${CC[(op >> 3) & 3]}, ${relTarget(pc, b1)}`);
  if ((op & 0xe7) === 0xc2) return three(`JP ${CC[(op >> 3) & 3]}, $${hex(word, 4)}`);
  if ((op & 0xe7) === 0xc4) return three(`CALL ${CC[(op >> 3) & 3]}, $${hex(word, 4)}`);
  if ((op & 0xe7) === 0xc0) return one(`RET ${CC[(op >> 3) & 3]}`);
  if ((op & 0xc7) === 0xc7) return one(`RST $${hex(op & 0x38)}`);
  if ((op & 0xcf) === 0xc1) return one(`POP ${RP2[(op >> 4) & 3]}`);
  if ((op & 0xcf) === 0xc5) return one(`PUSH ${RP2[(op >> 4) & 3]}`);
  return one(`.db $${hex(op)}`);
}

export function hexDump(bytes: Uint8Array, offset: number, length = 128): { offset: number; hex: string; ascii: string }[] {
  const rows: { offset: number; hex: string; ascii: string }[] = [];
  const end = Math.min(bytes.length, Math.max(0, offset) + length);
  for (let i = Math.max(0, offset); i < end; i += 16) {
    let hexPart = '';
    let ascii = '';
    for (let j = 0; j < 16; j += 1) {
      if (i + j < end) {
        const b = bytes[i + j];
        hexPart += `${hex(b)} `;
        ascii += b >= 32 && b < 127 ? String.fromCharCode(b) : '.';
      } else hexPart += '   ';
    }
    rows.push({ offset: i, hex: hexPart.trimEnd(), ascii });
  }
  return rows;
}

export type CpuView = '6502' | 'lr35902' | 'hex';

export function disassemble(bytes: Uint8Array, cpu: CpuView, address: number, count = 48): AsmLine[] {
  if (cpu === 'lr35902') return disassembleLr35902(bytes, address, count);
  if (cpu === '6502') return disassemble6502(bytes, address, count);
  return [];
}
