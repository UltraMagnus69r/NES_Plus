//! Linear 6502 disassembly for NES program data.
//! It does not follow banks, and it will misread data as code.
//! Game scripts are custom per game. This is not a script decompiler.

#[derive(Clone, Copy)]
enum Mode {
    Imp,
    Acc,
    Imm,
    Zp,
    Zpx,
    Zpy,
    Rel,
    Abs,
    Abx,
    Aby,
    Ind,
    Izx,
    Izy,
}

fn len(mode: Mode) -> usize {
    match mode {
        Mode::Imp | Mode::Acc => 1,
        Mode::Imm | Mode::Zp | Mode::Zpx | Mode::Zpy | Mode::Rel | Mode::Izx | Mode::Izy => 2,
        Mode::Abs | Mode::Abx | Mode::Aby | Mode::Ind => 3,
    }
}

fn meta(op: u8) -> Option<(&'static str, Mode)> {
    Some(match op {
        0x00 => ("BRK", Mode::Imp),
        0x01 => ("ORA", Mode::Izx),
        0x05 => ("ORA", Mode::Zp),
        0x06 => ("ASL", Mode::Zp),
        0x08 => ("PHP", Mode::Imp),
        0x09 => ("ORA", Mode::Imm),
        0x0a => ("ASL", Mode::Acc),
        0x0d => ("ORA", Mode::Abs),
        0x0e => ("ASL", Mode::Abs),
        0x10 => ("BPL", Mode::Rel),
        0x11 => ("ORA", Mode::Izy),
        0x15 => ("ORA", Mode::Zpx),
        0x16 => ("ASL", Mode::Zpx),
        0x18 => ("CLC", Mode::Imp),
        0x19 => ("ORA", Mode::Aby),
        0x1d => ("ORA", Mode::Abx),
        0x1e => ("ASL", Mode::Abx),
        0x20 => ("JSR", Mode::Abs),
        0x21 => ("AND", Mode::Izx),
        0x24 => ("BIT", Mode::Zp),
        0x25 => ("AND", Mode::Zp),
        0x26 => ("ROL", Mode::Zp),
        0x28 => ("PLP", Mode::Imp),
        0x29 => ("AND", Mode::Imm),
        0x2a => ("ROL", Mode::Acc),
        0x2c => ("BIT", Mode::Abs),
        0x2d => ("AND", Mode::Abs),
        0x2e => ("ROL", Mode::Abs),
        0x30 => ("BMI", Mode::Rel),
        0x31 => ("AND", Mode::Izy),
        0x35 => ("AND", Mode::Zpx),
        0x36 => ("ROL", Mode::Zpx),
        0x38 => ("SEC", Mode::Imp),
        0x39 => ("AND", Mode::Aby),
        0x3d => ("AND", Mode::Abx),
        0x3e => ("ROL", Mode::Abx),
        0x40 => ("RTI", Mode::Imp),
        0x41 => ("EOR", Mode::Izx),
        0x45 => ("EOR", Mode::Zp),
        0x46 => ("LSR", Mode::Zp),
        0x48 => ("PHA", Mode::Imp),
        0x49 => ("EOR", Mode::Imm),
        0x4a => ("LSR", Mode::Acc),
        0x4c => ("JMP", Mode::Abs),
        0x4d => ("EOR", Mode::Abs),
        0x4e => ("LSR", Mode::Abs),
        0x50 => ("BVC", Mode::Rel),
        0x51 => ("EOR", Mode::Izy),
        0x55 => ("EOR", Mode::Zpx),
        0x56 => ("LSR", Mode::Zpx),
        0x58 => ("CLI", Mode::Imp),
        0x59 => ("EOR", Mode::Aby),
        0x5d => ("EOR", Mode::Abx),
        0x5e => ("LSR", Mode::Abx),
        0x60 => ("RTS", Mode::Imp),
        0x61 => ("ADC", Mode::Izx),
        0x65 => ("ADC", Mode::Zp),
        0x66 => ("ROR", Mode::Zp),
        0x68 => ("PLA", Mode::Imp),
        0x69 => ("ADC", Mode::Imm),
        0x6a => ("ROR", Mode::Acc),
        0x6c => ("JMP", Mode::Ind),
        0x6d => ("ADC", Mode::Abs),
        0x6e => ("ROR", Mode::Abs),
        0x70 => ("BVS", Mode::Rel),
        0x71 => ("ADC", Mode::Izy),
        0x75 => ("ADC", Mode::Zpx),
        0x76 => ("ROR", Mode::Zpx),
        0x78 => ("SEI", Mode::Imp),
        0x79 => ("ADC", Mode::Aby),
        0x7d => ("ADC", Mode::Abx),
        0x7e => ("ROR", Mode::Abx),
        0x81 => ("STA", Mode::Izx),
        0x84 => ("STY", Mode::Zp),
        0x85 => ("STA", Mode::Zp),
        0x86 => ("STX", Mode::Zp),
        0x88 => ("DEY", Mode::Imp),
        0x8a => ("TXA", Mode::Imp),
        0x8c => ("STY", Mode::Abs),
        0x8d => ("STA", Mode::Abs),
        0x8e => ("STX", Mode::Abs),
        0x90 => ("BCC", Mode::Rel),
        0x91 => ("STA", Mode::Izy),
        0x94 => ("STY", Mode::Zpx),
        0x95 => ("STA", Mode::Zpx),
        0x96 => ("STX", Mode::Zpy),
        0x98 => ("TYA", Mode::Imp),
        0x99 => ("STA", Mode::Aby),
        0x9a => ("TXS", Mode::Imp),
        0x9d => ("STA", Mode::Abx),
        0xa0 => ("LDY", Mode::Imm),
        0xa1 => ("LDA", Mode::Izx),
        0xa2 => ("LDX", Mode::Imm),
        0xa4 => ("LDY", Mode::Zp),
        0xa5 => ("LDA", Mode::Zp),
        0xa6 => ("LDX", Mode::Zp),
        0xa8 => ("TAY", Mode::Imp),
        0xa9 => ("LDA", Mode::Imm),
        0xaa => ("TAX", Mode::Imp),
        0xac => ("LDY", Mode::Abs),
        0xad => ("LDA", Mode::Abs),
        0xae => ("LDX", Mode::Abs),
        0xb0 => ("BCS", Mode::Rel),
        0xb1 => ("LDA", Mode::Izy),
        0xb4 => ("LDY", Mode::Zpx),
        0xb5 => ("LDA", Mode::Zpx),
        0xb6 => ("LDX", Mode::Zpy),
        0xb8 => ("CLV", Mode::Imp),
        0xb9 => ("LDA", Mode::Aby),
        0xba => ("TSX", Mode::Imp),
        0xbc => ("LDY", Mode::Abx),
        0xbd => ("LDA", Mode::Abx),
        0xbe => ("LDX", Mode::Aby),
        0xc0 => ("CPY", Mode::Imm),
        0xc1 => ("CMP", Mode::Izx),
        0xc4 => ("CPY", Mode::Zp),
        0xc5 => ("CMP", Mode::Zp),
        0xc6 => ("DEC", Mode::Zp),
        0xc8 => ("INY", Mode::Imp),
        0xc9 => ("CMP", Mode::Imm),
        0xca => ("DEX", Mode::Imp),
        0xcc => ("CPY", Mode::Abs),
        0xcd => ("CMP", Mode::Abs),
        0xce => ("DEC", Mode::Abs),
        0xd0 => ("BNE", Mode::Rel),
        0xd1 => ("CMP", Mode::Izy),
        0xd5 => ("CMP", Mode::Zpx),
        0xd6 => ("DEC", Mode::Zpx),
        0xd8 => ("CLD", Mode::Imp),
        0xd9 => ("CMP", Mode::Aby),
        0xdd => ("CMP", Mode::Abx),
        0xde => ("DEC", Mode::Abx),
        0xe0 => ("CPX", Mode::Imm),
        0xe1 => ("SBC", Mode::Izx),
        0xe4 => ("CPX", Mode::Zp),
        0xe5 => ("SBC", Mode::Zp),
        0xe6 => ("INC", Mode::Zp),
        0xe8 => ("INX", Mode::Imp),
        0xe9 => ("SBC", Mode::Imm),
        0xea => ("NOP", Mode::Imp),
        0xec => ("CPX", Mode::Abs),
        0xed => ("SBC", Mode::Abs),
        0xee => ("INC", Mode::Abs),
        0xf0 => ("BEQ", Mode::Rel),
        0xf1 => ("SBC", Mode::Izy),
        0xf5 => ("SBC", Mode::Zpx),
        0xf6 => ("INC", Mode::Zpx),
        0xf8 => ("SED", Mode::Imp),
        0xf9 => ("SBC", Mode::Aby),
        0xfd => ("SBC", Mode::Abx),
        0xfe => ("INC", Mode::Abx),
        _ => return None,
    })
}

fn hex(n: u16, width: usize) -> String {
    format!("{n:0width$X}")
}

#[derive(Clone, Debug)]
pub struct AsmLine {
    pub address: u16,
    pub bytes: String,
    pub text: String,
}

/// Disassemble `count` instructions starting at `index` in `bytes`.
/// `cpu_addr` is the address shown for that first byte (for an NROM PRG, `$8000`).
pub fn disassemble6502(bytes: &[u8], index: usize, cpu_addr: u16, count: usize) -> Vec<AsmLine> {
    let mut lines = Vec::new();
    let mut pc = index;
    let mut addr = cpu_addr;
    for _ in 0..count {
        if pc >= bytes.len() {
            break;
        }
        let op = bytes[pc];
        let Some((name, mode)) = meta(op) else {
            lines.push(AsmLine {
                address: addr,
                bytes: hex(op as u16, 2),
                text: format!(".db ${}", hex(op as u16, 2)),
            });
            pc += 1;
            addr = addr.wrapping_add(1);
            continue;
        };
        let size = len(mode);
        if pc + size > bytes.len() {
            lines.push(AsmLine {
                address: addr,
                bytes: hex(op as u16, 2),
                text: format!(".db ${}", hex(op as u16, 2)),
            });
            break;
        }
        let raw = bytes[pc..pc + size]
            .iter()
            .map(|b| hex(*b as u16, 2))
            .collect::<Vec<_>>()
            .join(" ");
        let b1 = bytes[pc + if size > 1 { 1 } else { 0 }];
        let b2 = if size > 2 { bytes[pc + 2] } else { 0 };
        let abs = u16::from_le_bytes([b1, b2]);
        let arg = match mode {
            Mode::Imm => format!("#${}", hex(b1 as u16, 2)),
            Mode::Zp => format!("${}", hex(b1 as u16, 2)),
            Mode::Zpx => format!("${},X", hex(b1 as u16, 2)),
            Mode::Zpy => format!("${},Y", hex(b1 as u16, 2)),
            Mode::Abs => format!("${}", hex(abs, 4)),
            Mode::Abx => format!("${},X", hex(abs, 4)),
            Mode::Aby => format!("${},Y", hex(abs, 4)),
            Mode::Ind => format!("(${})", hex(abs, 4)),
            Mode::Izx => format!("(${},X)", hex(b1 as u16, 2)),
            Mode::Izy => format!("(${}),Y", hex(b1 as u16, 2)),
            Mode::Rel => {
                let rel = b1 as i8 as i16;
                let target = addr.wrapping_add(2).wrapping_add(rel as u16);
                format!("${}", hex(target, 4))
            }
            Mode::Acc => "A".into(),
            Mode::Imp => String::new(),
        };
        let text = if arg.is_empty() {
            name.to_string()
        } else {
            format!("{name} {arg}")
        };
        lines.push(AsmLine {
            address: addr,
            bytes: raw,
            text,
        });
        pc += size;
        addr = addr.wrapping_add(size as u16);
    }
    lines
}
