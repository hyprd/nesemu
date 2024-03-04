use bitflags::Flags;

use crate::opcodes;
use std::collections::HashMap;

struct CPU {
    reg_a: u8,
    reg_x: u8,
    reg_y: u8,
    reg_pc: u16,
    reg_sp: u8,
    reg_status: StatusFlags,
    memory: [u8; 0xFFFF],
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    IMM,
    ZP,
    ZP_X,
    ZP_Y,
    ABS,
    ABS_X,
    ABS_Y,
    IND_X,
    IND_Y,
    ACC,
    REL,
    IMP,
}

const STACK: u16 = 0x0100;

bitflags! {
    struct StatusFlags: u8 {
        const CARRY = 0b00000001;
        const ZERO = 0b00000010;
        const INTERRUPT_MASK = 0b00000100;
        const DECIMAL = 0b00001000;
        const BREAK = 0b00010000;
        const BREAK_2 = 0b00100000;
        const OVERFLOW = 0b01000000;
        const NEGATIVE = 0b10000000;
    }
}

impl CPU {
    fn new() -> Self {
        CPU {
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_pc: 0,
            reg_sp: 0xFD,
            reg_status: StatusFlags::from_bits_truncate(0b100100),
            memory: [0; 0xFFFF],
        }
    }
    fn resolve_addressing_mode(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::IMM => self.reg_pc,
            AddressingMode::ZP => self.mem_read(self.reg_pc) as u16,
            AddressingMode::ZP_X => {
                let base_address = self.mem_read(self.reg_pc);
                base_address.wrapping_add(self.reg_x) as u16
            }
            AddressingMode::ZP_Y => {
                let base_address = self.mem_read(self.reg_pc);
                base_address.wrapping_add(self.reg_y) as u16
            }
            AddressingMode::ABS => self.mem_read_u16(self.reg_pc),
            AddressingMode::ABS_X => {
                let base_address = self.mem_read_u16(self.reg_pc);
                base_address.wrapping_add(self.reg_x as u16)
            }
            AddressingMode::ABS_Y => {
                let base_address = self.mem_read_u16(self.reg_pc);
                base_address.wrapping_add(self.reg_y as u16)
            }
            AddressingMode::IND_X => {
                // IND, X -> Construct the address, then use it to reference
                // the memory location to load data from.
                let base_address = self.mem_read(self.reg_pc).wrapping_add(self.reg_x);
                let ll = self.mem_read(base_address as u16);
                let hh = self.mem_read(base_address.wrapping_add(1) as u16);
                (hh as u16) << 8 | (ll as u16)
            }
            AddressingMode::IND_Y => {
                // IND, Y -> Similar to IND, X but Y is added after constructing
                // the reference address.
                let base_address = self.mem_read(self.reg_pc);
                let ll = self.mem_read(base_address as u16);
                let hh = self.mem_read(base_address.wrapping_add(1) as u16);
                let llhh = (hh as u16) << 8 | (ll as u16);
                llhh.wrapping_add(self.reg_y as u16)
            }
            AddressingMode::ACC => self.reg_a as u16,
            AddressingMode::IMP => {
                panic!("Implement implied addressing");
            }
            AddressingMode::REL => {
                panic!("Implement relative addressing");
            }
        }
    }
    // Read from memory
    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }
    // Reading a word is done in little-endian format
    fn mem_read_u16(&mut self, addr: u16) -> u16 {
        // LL, HH are 6502 mnemonics
        let ll = self.mem_read(addr) as u16;
        let hh = self.mem_read(addr + 1) as u16;
        (hh << 8) | ll
    }
    // Reading a word is done in little-endian format
    fn mem_write_u16(&mut self, addr: u16, value: u16) {
        let hh = (value >> 8) as u8;
        let ll = (value & 0xFF) as u8;
        self.mem_write(addr, ll);
        self.mem_write(addr + 1, hh);
    }
    // Write to address
    fn mem_write(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }
    fn reset(&mut self) {
        self.reg_a = 0;
        self.reg_x = 0;
        self.reg_y = 0;
        self.reg_sp = 0xFD;
        self.reg_status = StatusFlags::from_bits_truncate(0b100100);
        self.reg_pc = self.mem_read_u16(0xFFFC);
    }
    // Load program from PRG ROM
    fn mem_load_prg(&mut self, cart: Vec<u8>) {
        // 0x8000 -> 0xFFFF is reserved for PRG ROM
        self.memory[0x8000..(0x8000 + cart.len())].copy_from_slice(&cart[..]);
        // NES re/initializes PC to the value @0xFFFC on RST
        self.mem_write_u16(0xFFFC, 0x8000);
    }
    // Run program loaded from PRG ROM
    fn mem_run_prg(&mut self, cart: Vec<u8>) {
        self.mem_load_prg(cart);
        self.reset();
        self.execute();
    }

    // Stack grows DOWNWARD in 6502 (and variants).
    fn stack_push(&mut self, value: u8) {
        self.mem_write((STACK as u16) + (self.reg_sp as u16), value);
        self.reg_sp = self.reg_sp.wrapping_sub(1);
    }

    fn stack_push_u16(&mut self, value: u16) {
        let hh = (value >> 8) as u8;
        let ll = (value & 0xFF) as u8;
        self.stack_push(hh);
        self.stack_push(ll);
    }

    fn stack_pop(&mut self) -> u8 {
        self.reg_sp = self.reg_sp.wrapping_add(1);
        self.mem_read((STACK as u16) + (self.reg_sp as u16))
    }

    fn stack_pop_u16(&mut self) {
        self.stack_pop();
        self.stack_pop();
    }
    fn execute(&mut self) {
        let ref jmp_table: HashMap<u8, &'static opcodes::Opcode> = *opcodes::OPCODES_JMP_TABLE;
        loop {
            let opcode = self.mem_read(self.reg_pc);
            self.reg_pc += 1;
            let instruction = jmp_table.get(&opcode).unwrap();
            let pc_snapshot = self.reg_pc;
            match opcode {
                0xA9 | 0xAD | 0xBD | 0xB9 | 0xA5 | 0xB5 | 0xA1 | 0xB1 => {
                    self.lda(&instruction.addressing_mode)
                }
                0xA2 | 0xAE | 0xBE | 0xA6 | 0xB6 => self.ldx(&instruction.addressing_mode),
                0xA0 | 0xAC | 0xBC | 0xA4 | 0xB4 => self.ldy(&instruction.addressing_mode),
                0x8D | 0x9D | 0x99 | 0x85 | 0x95 | 0x81 | 0x91 => {
                    self.sta(&instruction.addressing_mode)
                }
                0x8E | 0x86 | 0x96 => self.stx(&instruction.addressing_mode),
                0x8C | 0x84 | 0x94 => self.sty(&instruction.addressing_mode),
                0xAA => self.tax(),
                0xA8 => self.tay(),
                0xBA => self.tsx(),
                0x8A => self.txa(),
                0x9A => self.txs(),
                0x98 => self.tya(),
                0x48 => self.pha(),
                0x08 => self.php(),
                0x68 => self.pla(),
                0x28 => self.plp(),
                0x0A | 0x0E | 0x1E | 0x06 | 0x16 => self.asl(&instruction.addressing_mode),
                0x4A | 0x4E | 0x5E | 0x46 | 0x56 => self.lsr(&instruction.addressing_mode),
                0x2A | 0x2E | 0x3E | 0x26 | 0x36 => self.rol(&instruction.addressing_mode),
                0x6A | 0x6E | 0x7E | 0x66 | 0x76 => self.ror(&instruction.addressing_mode),
                0x29 | 0x2D | 0x3D | 0x39 | 0x25 | 0x35 | 0x21 | 0x31 => {
                    self.and(&instruction.addressing_mode)
                }
                0x2C | 0x24 => self.bit(&instruction.addressing_mode),
                0x49 | 0x4D | 0x5D | 0x59 | 0x45 | 0x55 | 0x41 | 0x51 => {
                    self.eor(&instruction.addressing_mode)
                }
                0x09 | 0x0D | 0x1D | 0x19 | 0x05 | 0x15 | 0x01 | 0x11 => {
                    self.ora(&instruction.addressing_mode)
                }
                0x69 | 0x6D | 0x7D | 0x79 | 0x65 | 0x75 | 0x61 | 0x71 => {
                    self.adc(&instruction.addressing_mode)
                }
                0xC9 | 0xCD | 0xDD | 0xD9 | 0xC5 | 0xD5 | 0xC1 | 0xD1 => {
                    self.cmp(&instruction.addressing_mode, self.reg_a)
                }
                0xE0 | 0xEC | 0xE4 => self.cpx(&instruction.addressing_mode),
                0xC0 | 0xCC | 0xC4 => self.cpy(&instruction.addressing_mode),
                0xE9 | 0xED | 0xFD | 0xF9 | 0xE5 | 0xF5 | 0xE1 | 0xF1 => {
                    self.sbc(&instruction.addressing_mode)
                }
                0xCE | 0xDE | 0xC6 | 0xD6 => self.dec(&instruction.addressing_mode),
                0xCA => self.dex(),
                0x88 => self.dey(),
                0xEE | 0xFE | 0xE6 | 0xF6 => self.inc(&instruction.addressing_mode),
                0xE8 => self.inx(),
                0xC8 => self.iny(),
                0x00 => {
                    self.brk();
                    break;
                }
                0x4C | 0x6C => self.jmp(&instruction.addressing_mode),
                0x20 => self.jsr(),
                0x40 => self.rti(),
                0x60 => self.rts(),
                0x90 => self.bcc(),
                0xB0 => self.bcs(),
                0xF0 => self.beq(),
                0x30 => self.bmi(),
                0xD0 => self.bne(),
                0x10 => self.bpl(),
                0x50 => self.bvc(),
                0x70 => self.bvs(),
                0x18 => self.clc(),
                0xD8 => self.cld(),
                0x58 => self.cli(),
                0xB8 => self.clv(),
                0x38 => self.sec(),
                0xF8 => self.sed(),
                0x78 => self.sei(),
                0xEA => self.nop(),
                _ => {
                    return;
                }
            }
            if pc_snapshot == self.reg_pc {
                self.reg_pc += (&instruction.length - 1) as u16;
            }
        }
    }

    fn handle_flags_z_n(&mut self, value: u8) {
        if value == 0 {
            self.reg_status.insert(StatusFlags::ZERO);
        } else {
            self.reg_status.remove(StatusFlags::ZERO);
        }

        if value & 0b10000000 != 0 {
            self.reg_status.insert(StatusFlags::NEGATIVE);
        } else {
            self.reg_status.remove(StatusFlags::NEGATIVE);
        }
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.reg_a = value;
        self.handle_flags_z_n(value);
    }
    fn ldx(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.reg_x = value;
        self.handle_flags_z_n(value);
    }
    fn ldy(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.reg_y = value;
        self.handle_flags_z_n(value);
    }
    fn sta(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        println!("[STA ADDRESS {:#02x?} REG A {:?}]", address, self.reg_a);
        self.mem_write(address, self.reg_a);
    }
    fn stx(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        self.mem_write(address, self.reg_x);
    }
    fn sty(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        self.mem_write(address, self.reg_y);
    }
    fn tax(&mut self) {
        self.reg_x = self.reg_a;
        self.handle_flags_z_n(self.reg_x);
    }
    fn tay(&mut self) {
        self.reg_y = self.reg_a;
        self.handle_flags_z_n(self.reg_y);
    }
    fn tsx(&mut self) {
        self.reg_x = self.reg_sp;
        self.handle_flags_z_n(self.reg_x);
    }
    fn txa(&mut self) {
        self.reg_a = self.reg_x;
        self.handle_flags_z_n(self.reg_a);
    }
    fn txs(&mut self) {
        self.reg_sp = self.reg_x;
    }
    fn tya(&mut self) {
        self.reg_a = self.reg_y;
    }
    fn pha(&mut self) {}
    fn php(&mut self) {}
    fn pla(&mut self) {}
    fn plp(&mut self) {}
    fn asl_a(&mut self) {
        let mut value = self.reg_a;
        if value >> 7 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.insert(StatusFlags::CARRY);
        }
        self.reg_a <<= 1;
    }
    fn asl(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address);
        if value >> 7 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        value <<= 1;
        self.mem_write(address, value);
        self.handle_flags_z_n(value);
    }
    fn lsr_a(&mut self) {
        let mut value = self.reg_a;
        if value & 0x01 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        self.reg_a >>= 1;
    }
    fn lsr(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        if value & 0x01 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
    }
    fn rol_a(&mut self) {
        let mut value = self.reg_a;
        let carry_flag = self.reg_status.contains(StatusFlags::CARRY);
        if value >> 7 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        if carry_flag {
            value |= 0x01;
        }
        self.reg_a = value;
    }
    fn rol(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address);
        let carry_flag = self.reg_status.contains(StatusFlags::CARRY);
        if value >> 7 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        value <<= 1;
        if carry_flag {
            value |= 1;
        }
        self.mem_write(address, value);
        self.handle_flags_z_n(value);
    }
    fn ror_a(&mut self) {
        let mut value = self.reg_a;
        let carry_flag = self.reg_status.contains(StatusFlags::CARRY);
        if value & 0x01 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        value >>= 1;
        if carry_flag {
            value |= 0b10000000;
        }
        self.reg_a = value;
    }
    fn ror(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address);
        let carry_flag = self.reg_status.contains(StatusFlags::CARRY);
        if value & 0x01 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        value >>= 1;
        if carry_flag {
            value |= 0b10000000;
        }
        self.mem_write(address, value);
        self.handle_flags_z_n(value);
    }
    fn and(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.reg_a &= value;
    }
    fn bit(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        if self.reg_a & value == 0 {
            self.reg_status.insert(StatusFlags::ZERO);
        } else {
            self.reg_status.remove(StatusFlags::ZERO);
        }
        // if bit 7 is set in value, set in reg_status
        self.reg_status
            .set(StatusFlags::NEGATIVE, value & 0b10000000 > 0);
        // if bit 6 is set in value, set in reg_status
        self.reg_status
            .set(StatusFlags::OVERFLOW, value & 0b01000000 > 0);
    }
    fn eor(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.reg_a ^= value;
    }
    fn ora(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.reg_a |= value;
    }
    fn cmp(&mut self, mode: &AddressingMode, compare: u8) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        if value <= compare {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY)
        }
        self.handle_flags_z_n(compare.wrapping_sub(value));
    }
    fn cpx(&mut self, mode: &AddressingMode) {
        self.cmp(mode, self.reg_x);
    }
    fn cpy(&mut self, mode: &AddressingMode) {
        self.cmp(mode, self.reg_y);
    }
    fn adc(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        let a = self.reg_a as u16;
        let mut carry = 0 as u16;
        if self.reg_status.contains(StatusFlags::CARRY) {
            carry = 1;
        }
        let evaluation = a ^ value as u16 ^ carry;
        if evaluation > 0xFF {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        // if overflows s8
        if (value ^ (evaluation as u8)) & ((evaluation as u8) ^ self.reg_a) & 0x80 != 0 {
            self.reg_status.insert(StatusFlags::OVERFLOW);
        } else {
            self.reg_status.remove(StatusFlags::OVERFLOW);
        }
        self.reg_a = evaluation as u8;
    }
    fn sbc(&mut self, mode: &AddressingMode) {}
    fn dec(&mut self, mode: &AddressingMode) {}
    fn dex(&mut self) {}
    fn dey(&mut self) {}
    fn inc(&mut self, _mode: &AddressingMode) {}
    fn inx(&mut self) {}
    fn iny(&mut self) {}
    fn brk(&mut self) {}
    fn jmp(&mut self, _mode: &AddressingMode) {}
    fn jsr(&mut self) {}
    fn rti(&mut self) {}
    fn rts(&mut self) {}
    fn bcc(&mut self) {}
    fn bcs(&mut self) {}
    fn beq(&mut self) {}
    fn bmi(&mut self) {}
    fn bne(&mut self) {}
    fn bpl(&mut self) {}
    fn bvc(&mut self) {}
    fn bvs(&mut self) {}
    fn clc(&mut self) {}
    fn cld(&mut self) {}
    fn cli(&mut self) {}
    fn clv(&mut self) {}
    fn sec(&mut self) {}
    fn sed(&mut self) {}
    fn sei(&mut self) {}
    fn nop(&mut self) {}
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn lda_imm() {
        let mut cpu = CPU::new();
        let cart = vec![0xA9, 0x05, 0x00];
        cpu.mem_run_prg(cart);
        assert_eq!(cpu.reg_a, 0x05);
    }
    #[test]
    fn ldx_imm() {
        let mut cpu = CPU::new();
        let cart = vec![0xA2, 0x05, 0x00];
        cpu.mem_run_prg(cart);
        assert_eq!(cpu.reg_x, 0x05);
    }
    #[test]
    fn ldy_imm() {
        let mut cpu = CPU::new();
        let cart = vec![0xA0, 0x05, 0x00];
        cpu.mem_run_prg(cart);
        assert_eq!(cpu.reg_y, 0x05);
    }

    #[test]
    fn sta_abs() {
        let mut cpu = CPU::new();
        let cart = vec![0x8D, 0x50, 0x50, 0x00];
        cpu.reg_a = 0xFF;
        cpu.mem_load_prg(cart);
        cpu.reg_pc = cpu.mem_read_u16(0xFFFC);
        cpu.execute();
        assert_eq!(cpu.mem_read(0x5050), 0xFF);
    }
    #[test]
    fn stx_abs() {
        let mut cpu = CPU::new();
        let cart = vec![0x8E, 0x50, 0x50, 0x00];
        cpu.reg_x = 0xFF;
        cpu.mem_load_prg(cart);
        cpu.reg_pc = cpu.mem_read_u16(0xFFFC);
        cpu.execute();
        assert_eq!(cpu.mem_read(0x5050), 0xFF);
    }
    #[test]
    fn sty_abs() {
        let mut cpu = CPU::new();
        let cart = vec![0x8C, 0x50, 0x50, 0x00];
        cpu.reg_y = 0xFF;
        cpu.mem_load_prg(cart);
        cpu.reg_pc = cpu.mem_read_u16(0xFFFC);
        cpu.execute();
        assert_eq!(cpu.mem_read(0x5050), 0xFF);
    }

    #[test]
    fn tax_imp() {
        let mut cpu = CPU::new();
        let cart = vec![0xAA, 0x00];
        cpu.reg_a = 0xFF;
        cpu.mem_load_prg(cart);
        cpu.reg_pc = cpu.mem_read_u16(0xFFFC);
        cpu.execute();
        assert_eq!(cpu.reg_x, 0xFF);
    }
    #[test]
    fn tay_imp() {
        let mut cpu = CPU::new();
        let cart = vec![0xA8, 0x00];
        cpu.reg_a = 0xFF;
        cpu.mem_load_prg(cart);
        cpu.reg_pc = cpu.mem_read_u16(0xFFFC);
        cpu.execute();
        assert_eq!(cpu.reg_y, 0xFF);
    }
    #[test]
    fn tsx_imp() {
        let mut cpu = CPU::new();
        let cart = vec![0xBA, 0x00];
        cpu.reg_sp = 0xFF;
        cpu.mem_load_prg(cart);
        cpu.reg_pc = cpu.mem_read_u16(0xFFFC);
        cpu.execute();
        assert_eq!(cpu.reg_x, 0xFF);
    }
    #[test]
    fn txa_imp() {
        let mut cpu = CPU::new();
        let cart = vec![0x8A, 0x00];
        cpu.reg_x = 0xFF;
        cpu.mem_load_prg(cart);
        cpu.reg_pc = cpu.mem_read_u16(0xFFFC);
        cpu.execute();
        assert_eq!(cpu.reg_a, 0xFF);
    }
    #[test]
    fn txs_imp() {
        let mut cpu = CPU::new();
        let cart = vec![0x9A, 0x00];
        cpu.reg_x = 0xFF;
        cpu.mem_load_prg(cart);
        cpu.reg_pc = cpu.mem_read_u16(0xFFFC);
        cpu.execute();
        assert_eq!(cpu.reg_sp, 0xFF);
    }
    #[test]
    fn tya_imp() {
        let mut cpu = CPU::new();
        let cart = vec![0x98, 0x00];
        cpu.reg_y = 0xFF;
        cpu.mem_load_prg(cart);
        cpu.reg_pc = cpu.mem_read_u16(0xFFFC);
        cpu.execute();
        assert_eq!(cpu.reg_a, 0xFF);
    }
}
