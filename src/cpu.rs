#![allow(warnings)]

use bitflags::Flags;

use crate::bus::Bus;
use crate::opcodes;
use std::collections::HashMap;

pub struct CPU {
    pub reg_a: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub reg_pc: u16,
    pub reg_sp: u8,
    pub reg_status: StatusFlags,
    pub bus: Bus,
    // memory: [u8; 0xFFFF],
}

#[derive(Debug, PartialEq)]
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
    #[derive(Clone)]
    pub struct StatusFlags: u8 {
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

pub trait Memory {
    fn mem_read(&mut self, addr: u16) -> u8;
    fn mem_write(&mut self, addr: u16, value: u8);
    fn mem_read_u16(&mut self, addr: u16) -> u16 {
        // LL, HH are 6502 mnemonics
        let ll = self.mem_read(addr) as u16;
        let hh = self.mem_read(addr + 1) as u16;
        (hh << 8) | (ll as u16)
    }
    fn mem_write_u16(&mut self, addr: u16, value: u16) {
        let hh = (value >> 8) as u8;
        let ll = (value & 0xFF) as u8;
        self.mem_write(addr, ll);
        self.mem_write(addr + 1, hh);
    }
}

impl Memory for CPU {
    fn mem_read(&mut self, addr: u16) -> u8 {
        self.bus.mem_read(addr)
    }
    fn mem_write(&mut self, addr: u16, value: u8) {
        self.bus.mem_write(addr, value)
    }
    fn mem_read_u16(&mut self, addr: u16) -> u16 {
        self.bus.mem_read_u16(addr)
    }
    fn mem_write_u16(&mut self, addr: u16, value: u16) {
        self.bus.mem_write_u16(addr, value)
    }
}

impl CPU {
    pub fn new(bus: Bus) -> Self {
        CPU {
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_pc: 0,
            reg_sp: 0xFD,
            reg_status: StatusFlags::from_bits_truncate(0b100100),
            // memory: [0; 0xFFFF],
            bus,
        }
    }
    pub fn resolve_addressing_mode(&mut self, mode: &AddressingMode) -> u16 {
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
            AddressingMode::IMP => 0,
            AddressingMode::REL => {
                panic!("Implement relative addressing");
            }
        }
    }
    pub fn reset(&mut self) {
        self.reg_a = 0x00;
        self.reg_x = 0x00;
        self.reg_y = 0x00;
        self.reg_sp = 0xFD;
        self.reg_status = StatusFlags::from_bits_truncate(0b100100);
        self.reg_pc = self.mem_read_u16(0xFFFC);
    }
    // Load program from PRG ROM
    pub fn load(&mut self, cart: Vec<u8>) {
        for i in 0..(cart.len() as u16) {
            self.mem_write(0x8600 + i, cart[i as usize]);
        }
        self.mem_write_u16(0xFFFC, 0x8600);
    }
    // Run program loaded from PRG ROM
    pub fn mem_run_prg(&mut self, cart: Vec<u8>) {
        self.load(cart);
        self.reset();
        self.run();
    }

    pub fn run(&mut self) {
        self.execute_with_callback(|_| {});
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

    fn stack_pop_u16(&mut self) -> u16 {
        let ll = self.stack_pop() as u16;
        let hh = self.stack_pop() as u16;
        hh << 8 | ll
    }
    // #  address R/W description
    // --- ------- --- -----------------------------------------------
    // 1    PC     R  fetch opcode (and discard it - $00 (BRK) is forced into the opcode register instead)
    // 2    PC     R  read next instruction byte (actually the same as above, since PC increment is suppressed. Also discarded.)
    // 3  $0100,S  W  push PCH on stack, decrement S
    // 4  $0100,S  W  push PCL on stack, decrement S
    // *** At this point, the signal status determines which interrupt vector is used ***
    // 5  $0100,S  W  push P on stack (with B flag *clear*), decrement S
    // 6   A       R  fetch PCL (A = FFFE for IRQ, A = FFFA for NMI), set I flag
    // 7   A       R  fetch PCH (A = FFFF for IRQ, A = FFFB for NMI)
    fn interrupt_nmi(&mut self) {
        self.stack_push_u16(self.reg_pc);
        let mut flags = self.reg_status.clone();
        flags.remove(StatusFlags::BREAK);
        flags.insert(StatusFlags::BREAK_2);
        self.stack_push(flags.bits());
        self.bus.tick(2);
        self.reg_pc = self.mem_read_u16(0xFFFA);
        self.reg_status.insert(StatusFlags::INTERRUPT_MASK);
    }

    pub fn execute_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        let ref jmp_table: HashMap<u8, &'static opcodes::Opcode> = *opcodes::OPCODES_JMP_TABLE;
        loop {
            if let Some(nmi) = self.bus.poll_nmi_status() {
                self.interrupt_nmi();
            }
            callback(self);
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
                0x0A => self.asl_a(),
                0x0E | 0x1E | 0x06 | 0x16 => self.asl(&instruction.addressing_mode),
                0x4A => self.lsr_a(),
                0x4E | 0x5E | 0x46 | 0x56 => self.lsr(&instruction.addressing_mode),
                0x2A => self.rol_a(),
                0x2E | 0x3E | 0x26 | 0x36 => self.rol(&instruction.addressing_mode),
                0x6A => self.ror_a(),
                0x6E | 0x7E | 0x66 | 0x76 => self.ror(&instruction.addressing_mode),
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
                0xE9 | 0xED | 0xEb | 0xFD | 0xF9 | 0xE5 | 0xF5 | 0xE1 | 0xF1 => {
                    self.sbc(&instruction.addressing_mode)
                }
                0xCE | 0xDE | 0xC6 | 0xD6 => self.dec(&instruction.addressing_mode),
                0xCA => self.dex(),
                0x88 => self.dey(),
                0xEE | 0xFE | 0xE6 | 0xF6 => self.inc(&instruction.addressing_mode),
                0xE8 => self.inx(),
                0xC8 => self.iny(),
                0x00 => {
                    // return;
                    self.brk();
                    break;
                }
                0x4C => self.reg_pc = self.mem_read_u16(self.reg_pc), // JMP ABS
                0x6C => self.jmp(),
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
                0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xEA | 0xFA | 0x80 | 0x82 | 0x89 | 0xC2
                | 0xE2 | 0x0C | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC | 0x04 | 0x44 | 0x64
                | 0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => self.nop(),
                0xAB | 0xAF | 0xBF | 0xA7 | 0xB7 | 0xA3 | 0xB3 => {
                    self.lax(&instruction.addressing_mode)
                }

                0x8F | 0x87 | 0x97 | 0x83 => self.sax(&instruction.addressing_mode),
                0xCF | 0xDF | 0xDB | 0xC7 | 0xD7 | 0xC3 | 0xD3 => {
                    self.dcp(&instruction.addressing_mode)
                }

                0xEF | 0xFF | 0xFB | 0xE7 | 0xF7 | 0xE3 | 0xF3 => {
                    self.isc(&instruction.addressing_mode)
                }

                0x0F | 0x1F | 0x1B | 0x07 | 0x17 | 0x03 | 0x13 => {
                    self.slo(&instruction.addressing_mode)
                }

                0x2F | 0x3F | 0x3B | 0x27 | 0x37 | 0x23 | 0x33 => {
                    self.rla(&instruction.addressing_mode)
                }

                0x4F | 0x5F | 0x5B | 0x47 | 0x57 | 0x43 | 0x53 => {
                    self.sre(&instruction.addressing_mode)
                }
                0x6F | 0x7F | 0x7B | 0x67 | 0x77 | 0x63 | 0x73 => {
                    self.rra(&instruction.addressing_mode)
                }
                _ => {
                    return;
                }
            }
            self.bus.tick(instruction.cycles);
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
    // Test the condition of a given flag. If set, branch
    // the PC to a signed value relative of the PC itself.
    // (-127 ~ +128 bytes after the branch instruction).
    fn branch(&mut self, flag_set: bool) {
        if flag_set {
            let jump_offset = self.mem_read(self.reg_pc) as i8;
            let address = self.reg_pc.wrapping_add(1).wrapping_add(jump_offset as u16);
            self.reg_pc = address;
        }
    }
    fn las(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        let evaluation = value & self.reg_sp;
        self.reg_a = evaluation;
        self.reg_x = evaluation;
        self.reg_sp = evaluation;
        self.handle_flags_z_n(evaluation);
    }
    fn rra(&mut self, mode: &AddressingMode) {
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
        self.add_to_a(value);
    }

    fn sre(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address);
        if value & 0x01 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        value >>= 1;
        self.mem_write(address, value);
        self.reg_a ^= value;
        self.handle_flags_z_n(self.reg_a);
    }

    fn rla(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address);
        let carry = self.reg_status.contains(StatusFlags::CARRY);
        if value >> 7 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        value <<= 1;
        if carry {
            value |= 1;
        }
        self.mem_write(address, value);
        if value >> 7 == 1 {
            self.reg_status.insert(StatusFlags::NEGATIVE);
        } else {
            self.reg_status.remove(StatusFlags::NEGATIVE);
        }
        self.reg_a &= value;
        self.handle_flags_z_n(self.reg_a);
    }
    fn slo(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address);
        if value >> 7 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        value <<= 1;
        self.mem_write(address, value);
        self.reg_a |= value;
        self.handle_flags_z_n(self.reg_a);
    }
    fn isc(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address);
        value = value.wrapping_add(1);
        self.mem_write(address, value);
        self.handle_flags_z_n(value);
        self.add_to_a(!value);
    }
    fn dcp(&mut self, mode: &AddressingMode) {
        // This instruction does not affect internal registers, so don't write
        // result to reg_a!
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address).wrapping_sub(1);
        self.mem_write(address, value);
        if value <= self.reg_a {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        self.handle_flags_z_n(self.reg_a.wrapping_sub(value));
    }
    fn lax(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.reg_a = value;
        self.reg_x = value;
        self.handle_flags_z_n(value);
    }
    fn sax(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value_a = self.reg_a;
        let value_x = self.reg_x;
        self.mem_write(address, value_a & value_x);
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
    fn pha(&mut self) {
        self.stack_push(self.reg_a);
    }
    fn php(&mut self) {
        let mut flags = self.reg_status.clone();
        flags.insert(StatusFlags::BREAK);
        flags.insert(StatusFlags::BREAK_2);
        self.stack_push(flags.bits());
    }
    fn pla(&mut self) {
        let accumulator_value = self.stack_pop();
        self.reg_a = accumulator_value;
        self.handle_flags_z_n(self.reg_a);
    }
    fn plp(&mut self) {
        let status_value = self.stack_pop();
        self.reg_status = CPU::dec_to_flags(status_value);
        self.reg_status.remove(StatusFlags::BREAK);
        self.reg_status.insert(StatusFlags::BREAK_2);
    }
    fn asl_a(&mut self) {
        let mut value = self.reg_a;
        if value >> 7 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        self.reg_a <<= 1;
        self.handle_flags_z_n(self.reg_a);
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
        self.reg_status.remove(StatusFlags::NEGATIVE);
        if self.reg_a == 0 {
            self.reg_status.insert(StatusFlags::ZERO);
        } else {
            self.reg_status.remove(StatusFlags::ZERO);
        }
    }
    fn lsr(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address);
        if value & 0x01 == 1 {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        value >>= 1;
        self.mem_write(address, value);
        self.reg_status.remove(StatusFlags::NEGATIVE);
        if value == 0 {
            self.reg_status.insert(StatusFlags::ZERO);
        } else {
            self.reg_status.remove(StatusFlags::ZERO);
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
        value <<= 1;
        if carry_flag {
            value |= 0x01;
        }
        self.reg_a = value;
        self.handle_flags_z_n(self.reg_a);
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
            value |= 0x01;
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
        self.handle_flags_z_n(self.reg_a);
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
        self.handle_flags_z_n(self.reg_a);
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
            .set(StatusFlags::OVERFLOW, value & 0b1000000 > 0);
    }
    fn eor(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.reg_a ^= value;
        self.handle_flags_z_n(self.reg_a);
    }
    fn ora(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.reg_a |= value;
        self.handle_flags_z_n(self.reg_a);
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

    // ADC can also be used for SBC operations as:
    //  -> A - B = A + (-B).
    //  -> -B = !B + 1
    // This is a generic function both operations can use.
    fn add_to_a(&mut self, value: u8) {
        let mut carry = 0;
        if self.reg_status.contains(StatusFlags::CARRY) {
            carry = 1;
        }
        let sum = self.reg_a as u16 + value as u16 + carry;
        if sum > 0xFF {
            self.reg_status.insert(StatusFlags::CARRY);
        } else {
            self.reg_status.remove(StatusFlags::CARRY);
        }
        let eval = sum as u8;
        if (value ^ eval) & (eval ^ self.reg_a) & 0x80 != 0 {
            self.reg_status.insert(StatusFlags::OVERFLOW);
        } else {
            self.reg_status.remove(StatusFlags::OVERFLOW)
        }
        self.reg_a = eval;
        self.handle_flags_z_n(self.reg_a);
    }

    fn adc(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.add_to_a(value);
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address) as i8;
        self.add_to_a(value.wrapping_neg().wrapping_sub(1) as u8);
    }
    fn dec(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address);
        value = value.wrapping_sub(1);
        self.mem_write(address, value);
        self.handle_flags_z_n(value);
    }
    fn dex(&mut self) {
        self.reg_x = self.reg_x.wrapping_sub(1);
        self.handle_flags_z_n(self.reg_x);
    }
    fn dey(&mut self) {
        self.reg_y = self.reg_y.wrapping_sub(1);
        self.handle_flags_z_n(self.reg_y);
    }
    fn inc(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let mut value = self.mem_read(address);
        value = value.wrapping_add(1);
        self.mem_write(address, value);
        self.handle_flags_z_n(value);
    }
    fn inx(&mut self) {
        self.reg_x = self.reg_x.wrapping_add(1);
        self.handle_flags_z_n(self.reg_x);
    }
    fn iny(&mut self) {
        self.reg_y = self.reg_y.wrapping_add(1);
        self.handle_flags_z_n(self.reg_y);
    }
    fn brk(&mut self) {}
    fn jmp(&mut self) {
        // Implementation of JMP IND.
        //
        // The 6502 has a bug (or feature) where wrapping the LSB
        // doesn't increment the MSB. This is because a 6502 cannot
        // do increments of 16-bit values within a single cycle.
        //
        // Indirect jumps must never use a vector beginning on the
        // last byte of a page.
        //
        // For example:
        // - 0x3000 = 0x40
        // - 0x30FF = 0x80
        // - 0x3100 = 0x50
        //
        // The result of JMP (0x30FF) will transfer control to 0x4080
        // rather than 0x5080 as expected.
        //
        let address = self.mem_read_u16(self.reg_pc);
        if address & 0x00FF == 0x00FF {
            let ll = self.mem_read(address) as u16;
            let hh = self.mem_read(address & 0xFF00) as u16;
            self.reg_pc = (hh << 8) | ll;
        } else {
            self.reg_pc = self.mem_read_u16(address);
        }
    }
    fn jsr(&mut self) {
        // Return pointer on the stack to return to regular control flow
        // after the soubroutine is executed.
        self.stack_push_u16(self.reg_pc + 1);
        let address = self.mem_read_u16(self.reg_pc);
        self.reg_pc = address;
    }
    fn dec_to_flags(value: u8) -> StatusFlags {
        StatusFlags::from_bits_truncate(value)
    }
    fn rti(&mut self) {
        let flags = self.stack_pop();
        self.reg_status = CPU::dec_to_flags(flags);
        self.reg_status.remove(StatusFlags::BREAK);
        self.reg_status.insert(StatusFlags::BREAK_2);
        self.reg_pc = self.stack_pop_u16();
    }
    fn rts(&mut self) {
        let data = self.stack_pop_u16();
        self.reg_pc = data + 1;
    }
    fn bcc(&mut self) {
        self.branch(!self.reg_status.contains(StatusFlags::CARRY));
    }
    fn bcs(&mut self) {
        self.branch(self.reg_status.contains(StatusFlags::CARRY));
    }
    fn beq(&mut self) {
        self.branch(self.reg_status.contains(StatusFlags::ZERO));
    }
    fn bne(&mut self) {
        self.branch(!self.reg_status.contains(StatusFlags::ZERO));
    }
    fn bmi(&mut self) {
        self.branch(self.reg_status.contains(StatusFlags::NEGATIVE));
    }
    fn bpl(&mut self) {
        self.branch(!self.reg_status.contains(StatusFlags::NEGATIVE));
    }
    fn bvc(&mut self) {
        self.branch(!self.reg_status.contains(StatusFlags::OVERFLOW));
    }
    fn bvs(&mut self) {
        self.branch(self.reg_status.contains(StatusFlags::OVERFLOW));
    }
    fn clc(&mut self) {
        self.reg_status.remove(StatusFlags::CARRY);
    }
    fn cld(&mut self) {
        self.reg_status.remove(StatusFlags::DECIMAL);
    }
    fn cli(&mut self) {
        self.reg_status.remove(StatusFlags::INTERRUPT_MASK);
    }
    fn clv(&mut self) {
        self.reg_status.remove(StatusFlags::OVERFLOW);
    }
    fn sec(&mut self) {
        self.reg_status.insert(StatusFlags::CARRY);
    }
    fn sed(&mut self) {
        self.reg_status.insert(StatusFlags::DECIMAL);
    }
    fn sei(&mut self) {
        self.reg_status.insert(StatusFlags::INTERRUPT_MASK);
    }
    fn nop(&mut self) {}
}

enum TestRegister {
    A,
    X,
    Y,
    PS,
    SP,
}

#[derive(PartialEq)]
enum AssertionType {
    Memory,
    Register,
    Stack,
}

enum InstructionType {
    LOAD,
    TRANSFER,
    STACK,
    SHIFT,
    LOGIC,
    ARITHMETIC,
    INCREMENT,
    CONTROL,
    BRANCH,
    FLAG,
}

// Host function for setting up and running an instruction test
//fn test_instruction(inst_type: InstructionType, inst: u8, addr_mode: AddressingMode) -> CPU {
//    let bus = Bus::new(test::test_rom());
//    let mut cpu = CPU::new(bus);
//    cpu.mem_load_prg(generate_test_cart(inst, addr_mode));
//    preallocate_cpu_values(&mut cpu, inst_type);
//    cpu.reg_pc = cpu.mem_read_u16(0xFFFC);
//    cpu.run();
//    cpu
//}

////// Some registers need some values preallocated in registers or memory
////// before execution in order to test properly
//fn preallocate_cpu_values(cpu: &mut CPU, inst_type: InstructionType) {
//    match inst_type {
//        InstructionType::LOAD => {
//            cpu.reg_a = 0x10;
//            cpu.reg_x = 0x10;
//            cpu.reg_y = 0x10;
//            // Zero Page
//            cpu.mem_write(0x10, 0x20);
//            // Zero Page X, Y
//            cpu.mem_write(0x20, 0x20);
//            // Absolute
//            cpu.mem_write(0xF0A0, 0x20);
//            cpu.mem_write(0xF0B0, 0x20);
//            // Indirect X
//            cpu.mem_write(0x52, 0x20);
//            cpu.mem_write(0x53, 0x20);
//            cpu.mem_write(0x2020, 0x20);
//            // Indirect Y
//            cpu.mem_write(0x42, 0x20);
//            cpu.mem_write(0x43, 0x20);
//            cpu.mem_write(0x2030, 0x20);
//        }
//        InstructionType::TRANSFER => {
//            cpu.reg_a = 0x10;
//            cpu.reg_sp = 0x10;
//            cpu.reg_x = 0x10;
//            cpu.reg_y = 0x10;
//            // Indirect X
//            cpu.mem_write(0x52, 0x20);
//            cpu.mem_write(0x53, 0x20);
//            // Indirect Y
//            cpu.mem_write(0x42, 0x20);
//            cpu.mem_write(0x43, 0x20);
//        }
//        InstructionType::STACK => {
//            cpu.reg_a = 0x10;
//            cpu.reg_status.insert(StatusFlags::INTERRUPT_MASK);
//            cpu.stack_push(0x16);
//        }
//        InstructionType::SHIFT => {
//            cpu.reg_a = 0x10;
//            // Addressing modes use this register
//            cpu.reg_x = 0x10;
//            // Zero Page
//            cpu.mem_write(0x10, 0x20);
//            // Zero Page X
//            cpu.mem_write(0x20, 0x20);
//            // Absolute
//            cpu.mem_write(0xF0A0, 0x20);
//            cpu.mem_write(0xF0B0, 0x20);
//        }
//        InstructionType::LOGIC => {
//            cpu.reg_a = 0xB6;
//            // Addressing modes use these two registers
//            cpu.reg_x = 0x10;
//            cpu.reg_y = 0x10;
//            // Zero Page
//            cpu.mem_write(0x10, 0x80);
//            // Zero Page X
//            cpu.mem_write(0x20, 0x80);
//            // Absolute
//            cpu.mem_write(0xF0B0, 0x80);
//            cpu.mem_write(0xF0A0, 0x80);
//            // Indirect X
//            cpu.mem_write(0x52, 0x20);
//            cpu.mem_write(0x53, 0x20);
//            cpu.mem_write(0x2020, 0x80);
//            // Indirect Y
//            cpu.mem_write(0x42, 0x20);
//            cpu.mem_write(0x43, 0x20);
//            cpu.mem_write(0x2030, 0x80);
//        }
//        InstructionType::ARITHMETIC => {
//            cpu.reg_a = 0x10;
//            cpu.reg_status.insert(StatusFlags::CARRY);
//            // Addressing modes use these two registers
//            cpu.reg_x = 0x10;
//            cpu.reg_y = 0x10;
//            // Zero Page
//            cpu.mem_write(0x10, 0x20);
//            // Absolute
//            cpu.mem_write(0xF0B0, 0x20);
//            cpu.mem_write(0xF0A0, 0x20);
//            // Indirect X
//            cpu.mem_write(0x52, 0x20);
//            cpu.mem_write(0x53, 0x20);
//            cpu.mem_write(0x2020, 0x20);
//            // Indirect Y
//            cpu.mem_write(0x42, 0x20);
//            cpu.mem_write(0x43, 0x20);
//            cpu.mem_write(0x2030, 0x20);
//        }
//        InstructionType::INCREMENT => {
//            cpu.reg_x = 0x02;
//            cpu.reg_y = 0x02;
//            // Zero Page
//            cpu.mem_write(0x10, 0x20);
//            // Absolute
//            cpu.mem_write(0xF0B0, 0x20);
//            cpu.mem_write(0xF0A0, 0x20);
//        }
//        InstructionType::CONTROL | InstructionType::BRANCH | InstructionType::FLAG => {}
//    }
//}

////// Generates the testing cartridge setup for an addressing mode
//fn generate_test_cart(inst: u8, addr_mode: AddressingMode) -> Vec<u8> {
//    let mut cart = vec![];
//    match addr_mode {
//        AddressingMode::IMM => {
//            cart = vec![inst, 0x20, 0x00];
//        }
//        AddressingMode::ZP | AddressingMode::ZP_X | AddressingMode::ZP_Y => {
//            cart = vec![inst, 0x10, 0x00];
//        }
//        AddressingMode::ABS | AddressingMode::ABS_X | AddressingMode::ABS_Y => {
//            // Remember little endian! address = 0xF0A0
//            cart = vec![inst, 0xA0, 0xF0, 0x00];
//        }
//        AddressingMode::IND_X | AddressingMode::IND_Y => {
//            cart = vec![inst, 0x42, 0x00];
//        }
//        AddressingMode::ACC | AddressingMode::REL | AddressingMode::IMP => {
//            cart = vec![inst, 0x00];
//        }
//    };
//    cart
//}
//#[cfg(test)]
//mod test_cpu {
//    use super::*;
//    #[test]
//    fn lda_imm() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xA9, AddressingMode::IMM);
//        assert_eq!(cpu.reg_a, 0x20);
//    }
//    #[test]
//    fn lda_abs() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xAD, AddressingMode::ABS);
//        assert_eq!(cpu.reg_a, 0x20);
//    }
//    #[test]
//    fn lda_abs_x() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xBD, AddressingMode::ABS_X);
//        assert_eq!(cpu.reg_a, 0x20);
//    }
//    #[test]
//    fn lda_abs_y() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xB9, AddressingMode::ABS_Y);
//        assert_eq!(cpu.reg_a, 0x20);
//    }
//    #[test]
//    fn lda_zp() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xA5, AddressingMode::ZP);
//        assert_eq!(cpu.reg_a, 0x20);
//    }
//    #[test]
//    fn lda_zp_x() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xB5, AddressingMode::ZP_X);
//        assert_eq!(cpu.reg_a, 0x20);
//    }
//    #[test]
//    fn lda_ind_x() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xA1, AddressingMode::IND_X);
//        assert_eq!(cpu.reg_a, 0x20);
//    }
//    #[test]
//    fn lda_ind_y() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xB1, AddressingMode::IND_Y);
//        assert_eq!(cpu.reg_a, 0x20);
//    }
//    #[test]
//    fn ldx_imm() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xA2, AddressingMode::IMM);
//        assert_eq!(cpu.reg_x, 0x20);
//    }
//    #[test]
//    fn ldx_abs() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xAE, AddressingMode::ABS);
//        assert_eq!(cpu.reg_x, 0x20);
//    }
//    #[test]
//    fn ldx_abs_y() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xBE, AddressingMode::ABS_Y);
//        assert_eq!(cpu.reg_x, 0x20);
//    }
//    #[test]
//    fn ldx_zp() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xA6, AddressingMode::ZP);
//        assert_eq!(cpu.reg_x, 0x20);
//    }
//    #[test]
//    fn ldx_zp_y() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xB6, AddressingMode::ZP_Y);
//        assert_eq!(cpu.reg_x, 0x20);
//    }
//    #[test]
//    fn ldy_imm() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xA0, AddressingMode::IMM);
//        assert_eq!(cpu.reg_y, 0x20);
//    }
//    #[test]
//    fn ldy_abs() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xAC, AddressingMode::ABS);
//        assert_eq!(cpu.reg_y, 0x20);
//    }
//    #[test]
//    fn ldy_abs_y() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xBC, AddressingMode::ABS_Y);
//        assert_eq!(cpu.reg_y, 0x20);
//    }
//    #[test]
//    fn ldy_zp() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xA4, AddressingMode::ZP);
//        assert_eq!(cpu.reg_y, 0x20);
//    }
//    #[test]
//    fn ldy_zp_x() {
//        let cpu = test_instruction(InstructionType::LOAD, 0xB4, AddressingMode::ZP_X);
//        assert_eq!(cpu.reg_y, 0x20);
//    }
//    #[test]
//    fn sta_abs() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x8D, AddressingMode::ABS);
//        assert_eq!(cpu.mem_read(0xF0A0), cpu.reg_a);
//        assert_eq!(cpu.mem_read(0xF0A0), 0x10);
//    }
//    #[test]
//    fn sta_abs_x() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x9D, AddressingMode::ABS_X);
//        assert_eq!(cpu.mem_read(0xF0B0), cpu.reg_a);
//        assert_eq!(cpu.mem_read(0xF0B0), 0x10);
//    }
//    #[test]
//    fn sta_abs_y() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x99, AddressingMode::ABS_Y);
//        assert_eq!(cpu.mem_read(0xF0B0), cpu.reg_a);
//        assert_eq!(cpu.mem_read(0xF0B0), 0x10);
//    }
//    #[test]
//    fn sta_zp() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x85, AddressingMode::ZP);
//        assert_eq!(cpu.mem_read(0x10), cpu.reg_a);
//        assert_eq!(cpu.mem_read(0x10), 0x10);
//    }
//    #[test]
//    fn sta_zp_x() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x95, AddressingMode::ZP_X);
//        assert_eq!(cpu.mem_read(0x20), cpu.reg_a);
//        assert_eq!(cpu.mem_read(0x20), 0x10);
//    }
//    #[test]
//    fn sta_ind_x() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x81, AddressingMode::IND_X);
//        assert_eq!(cpu.mem_read(0x2020), cpu.reg_a);
//        assert_eq!(cpu.mem_read(0x2020), 0x10);
//    }
//    #[test]
//    fn sta_ind_y() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x91, AddressingMode::IND_Y);
//        assert_eq!(cpu.mem_read(0x2030), cpu.reg_a);
//        assert_eq!(cpu.mem_read(0x2030), 0x10);
//    }
//    #[test]
//    fn stx_abs() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x8E, AddressingMode::ABS);
//        assert_eq!(cpu.mem_read(0xF0A0), cpu.reg_x);
//        assert_eq!(cpu.mem_read(0xF0A0), 0x10);
//    }
//    #[test]
//    fn stx_zp() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x86, AddressingMode::ZP);
//        assert_eq!(cpu.mem_read(0x10), cpu.reg_x);
//        assert_eq!(cpu.mem_read(0x10), 0x10);
//    }
//    #[test]
//    fn stx_zp_y() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x96, AddressingMode::ZP_Y);
//        assert_eq!(cpu.mem_read(0x20), cpu.reg_x);
//        assert_eq!(cpu.mem_read(0x20), 0x10);
//    }
//    #[test]
//    fn sty_abs() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x8C, AddressingMode::ABS);
//        assert_eq!(cpu.mem_read(0xF0A0), cpu.reg_x);
//        assert_eq!(cpu.mem_read(0xF0A0), 0x10);
//    }
//    #[test]
//    fn sty_zp() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x84, AddressingMode::ZP);
//        assert_eq!(cpu.mem_read(0x10), cpu.reg_y);
//        assert_eq!(cpu.mem_read(0x10), 0x10);
//    }
//    #[test]
//    fn sty_zp_y() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x94, AddressingMode::ZP_X);
//        assert_eq!(cpu.mem_read(0x20), cpu.reg_y);
//        assert_eq!(cpu.mem_read(0x20), 0x10);
//    }
//    #[test]
//    fn tax_imp() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0xAA, AddressingMode::IMP);
//        // A and X are given the same value in test setup, so comparing them
//        // doesn't really verify the test environment ran successfully.
//        //
//        // We can atleast check the PC moved post instruction.
//        //
//        // Too lazy to change test setup values for a set of instructions that
//        // have straightforward execution anyway.
//        //
//        // Pretty much the same deal for the rest of the transfer instructions.
//        assert_eq!(cpu.reg_x, cpu.reg_a);
//        assert_eq!(cpu.reg_pc, 0x8002);
//    }
//    #[test]
//    fn tay_imp() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0xA8, AddressingMode::IMP);
//        assert_eq!(cpu.reg_y, cpu.reg_a);
//        assert_eq!(cpu.reg_pc, 0x8002);
//    }
//    #[test]
//    fn tsx_imp() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0xBA, AddressingMode::IMP);
//        assert_eq!(cpu.reg_x, cpu.reg_sp);
//        assert_eq!(cpu.reg_pc, 0x8002);
//    }
//    #[test]
//    fn txa_imp() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x8A, AddressingMode::IMP);
//        assert_eq!(cpu.reg_a, cpu.reg_x);
//        assert_eq!(cpu.reg_pc, 0x8002);
//    }
//    #[test]
//    fn txs_imp() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x9A, AddressingMode::IMP);
//        assert_eq!(cpu.reg_sp, cpu.reg_x);
//        assert_eq!(cpu.reg_pc, 0x8002);
//    }
//    #[test]
//    fn tya_imp() {
//        let cpu = test_instruction(InstructionType::TRANSFER, 0x98, AddressingMode::IMP);
//        assert_eq!(cpu.reg_a, cpu.reg_y);
//        assert_eq!(cpu.reg_pc, 0x8002);
//    }
//    #[test]
//    fn pha_imp() {
//        let cpu = test_instruction(InstructionType::STACK, 0x48, AddressingMode::IMP);
//        assert_eq!(cpu.mem_read(STACK + cpu.reg_sp as u16 + 1), cpu.reg_a);
//    }
//    #[test]
//    fn php_imp() {
//        let mut cpu = test_instruction(InstructionType::STACK, 0x08, AddressingMode::IMP);
//        // PHP clones reg_status then sets BRK and BRK_2, so they need to be set in the test too.
//        cpu.reg_status.insert(StatusFlags::BREAK);
//        cpu.reg_status.insert(StatusFlags::BREAK_2);
//        assert_eq!(
//            cpu.mem_read(STACK + cpu.reg_sp as u16 + 1),
//            cpu.reg_status.bits()
//        );
//    }
//    #[test]
//    fn pla_imp() {
//        let cpu = test_instruction(InstructionType::STACK, 0x68, AddressingMode::IMP);
//        assert_ne!(cpu.reg_a, 0x10);
//        assert_eq!(cpu.reg_a, 0x16);
//    }
//    #[test]
//    fn plp_imp() {
//        let cpu = test_instruction(InstructionType::STACK, 0x28, AddressingMode::IMP);
//        assert_eq!(cpu.reg_status.bits(), 0x16);
//    }
//    #[test]
//    fn asl_acc() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x0A, AddressingMode::ACC);
//        assert_ne!(cpu.reg_a, 0x10);
//        assert_eq!(cpu.reg_a, 0x20);
//    }
//    #[test]
//    fn asl_abs() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x0E, AddressingMode::ABS);
//        assert_eq!(cpu.mem_read(0xF0A0), 0x40);
//    }
//    #[test]
//    fn asl_abs_x() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x1E, AddressingMode::ABS_X);
//        assert_eq!(cpu.mem_read(0xF0B0), 0x40);
//    }
//    #[test]
//    fn asl_zp() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x06, AddressingMode::ZP);
//        assert_eq!(cpu.mem_read(0x10), 0x40);
//    }
//    #[test]
//    fn asl_zp_x() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x16, AddressingMode::ZP_X);
//        assert_eq!(cpu.mem_read(0x20), 0x40);
//    }
//    #[test]
//    fn lsr_acc() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x4A, AddressingMode::ACC);
//        assert_eq!(cpu.reg_a, 0x08);
//    }
//    #[test]
//    fn lsr_abs() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x4E, AddressingMode::ABS);
//        assert_eq!(cpu.mem_read(0xF0A0), 0x10);
//    }
//    #[test]
//    fn lsr_abs_x() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x5E, AddressingMode::ABS_X);
//        assert_eq!(cpu.mem_read(0xF0B0), 0x10);
//    }
//    #[test]
//    fn lsr_zp() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x46, AddressingMode::ZP);
//        assert_eq!(cpu.mem_read(0x10), 0x10);
//    }
//    #[test]
//    fn lsr_zp_x() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x56, AddressingMode::ZP_X);
//        assert_eq!(cpu.mem_read(0x20), 0x10);
//    }
//    #[test]
//    fn rol_acc() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x2A, AddressingMode::ACC);
//        assert_eq!(cpu.reg_a, 0x10);
//    }
//    #[test]
//    fn rol_abs() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x2E, AddressingMode::ABS);
//        assert_eq!(cpu.mem_read(0xF0A0), 0x40);
//    }
//    #[test]
//    fn rol_abs_x() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x3E, AddressingMode::ABS_X);
//        assert_eq!(cpu.mem_read(0xF0B0), 0x40);
//    }
//    #[test]
//    fn rol_zp() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x26, AddressingMode::ZP);
//        assert_eq!(cpu.mem_read(0x10), 0x40);
//    }
//    #[test]
//    fn rol_zp_x() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x36, AddressingMode::ZP_X);
//        assert_eq!(cpu.mem_read(0x20), 0x40);
//    }
//    #[test]
//    fn ror_acc() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x4A, AddressingMode::ACC);
//        assert_eq!(cpu.reg_a, 0x08);
//    }
//    #[test]
//    fn ror_abs() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x4E, AddressingMode::ABS);
//        assert_eq!(cpu.mem_read(0xF0A0), 0x10);
//    }
//    #[test]
//    fn ror_abs_x() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x5E, AddressingMode::ABS_X);
//        assert_eq!(cpu.mem_read(0xF0B0), 0x10);
//    }
//    #[test]
//    fn ror_zp() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x46, AddressingMode::ZP);
//        assert_eq!(cpu.mem_read(0x10), 0x10);
//    }
//    #[test]
//    fn ror_zp_x() {
//        let cpu = test_instruction(InstructionType::SHIFT, 0x56, AddressingMode::ZP_X);
//        assert_eq!(cpu.mem_read(0x20), 0x10);
//    }
//    #[test]
//    fn and_imm() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x29, AddressingMode::IMM);
//        assert_eq!(cpu.reg_a, 0x20);
//    }
//    #[test]
//    fn and_abs() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x2D, AddressingMode::ABS);
//        assert_eq!(cpu.reg_a, 0x80);
//    }
//    #[test]
//    fn and_abs_x() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x3D, AddressingMode::ABS_X);
//        assert_eq!(cpu.reg_a, 0x80);
//    }
//    #[test]
//    fn and_abs_y() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x39, AddressingMode::ABS_Y);
//        assert_eq!(cpu.reg_a, 0x80);
//    }
//    #[test]
//    fn and_zp() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x25, AddressingMode::ZP);
//        assert_eq!(cpu.reg_a, 0x80);
//    }
//    #[test]
//    fn and_zp_x() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x35, AddressingMode::ZP_X);
//        assert_eq!(cpu.reg_a, 0x80);
//    }
//    #[test]
//    fn and_ind_x() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x21, AddressingMode::IND_X);
//        assert_eq!(cpu.reg_a, 0x80);
//    }
//    #[test]
//    fn and_ind_y() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x31, AddressingMode::IND_Y);
//        assert_eq!(cpu.reg_a, 0x80);
//    }
//    #[test]
//    fn bit_abs() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x2C, AddressingMode::ABS);
//        // Whether NEGATIVE flag is not set (intended behaviour) in reg_status
//        assert_eq!(cpu.reg_status.contains(StatusFlags::ZERO), false);
//        // 0x80 (target value) sets the last bit, so NEGATIVE = true
//        assert_eq!(cpu.reg_status.contains(StatusFlags::NEGATIVE), true);
//        assert_eq!(cpu.reg_status.contains(StatusFlags::OVERFLOW), false);
//    }
//    #[test]
//    fn bit_zp() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x24, AddressingMode::ZP);
//        assert_eq!(cpu.reg_status.contains(StatusFlags::ZERO), false);
//        assert_eq!(cpu.reg_status.contains(StatusFlags::NEGATIVE), true);
//        assert_eq!(cpu.reg_status.contains(StatusFlags::OVERFLOW), false);
//    }
//    #[test]
//    fn eor_imm() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x49, AddressingMode::IMM);
//        assert_eq!(cpu.reg_a, 0x96);
//    }
//    #[test]
//    fn eor_abs() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x4D, AddressingMode::ABS);
//        assert_eq!(cpu.reg_a, 0x36);
//    }
//    #[test]
//    fn eor_abs_x() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x5D, AddressingMode::ABS_X);
//        assert_eq!(cpu.reg_a, 0x36);
//    }
//    #[test]
//    fn eor_abs_y() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x59, AddressingMode::ABS_Y);
//        assert_eq!(cpu.reg_a, 0x36);
//    }
//    #[test]
//    fn eor_zp() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x45, AddressingMode::ZP);
//        assert_eq!(cpu.reg_a, 0x36);
//    }
//    #[test]
//    fn eor_zp_x() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x55, AddressingMode::ZP_X);
//        assert_eq!(cpu.reg_a, 0x36);
//    }
//    #[test]
//    fn eor_ind_x() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x41, AddressingMode::IND_X);
//        assert_eq!(cpu.reg_a, 0x36);
//    }
//    #[test]
//    fn eor_ind_y() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x51, AddressingMode::IND_Y);
//        assert_eq!(cpu.reg_a, 0x36);
//    }
//    #[test]
//    fn ora_imm() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x09, AddressingMode::IMM);
//        assert_eq!(cpu.reg_a, 0xB6);
//    }
//    #[test]
//    fn ora_abs() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x0D, AddressingMode::ABS);
//        assert_eq!(cpu.reg_a, 0xB6);
//    }
//    #[test]
//    fn ora_abs_x() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x1D, AddressingMode::ABS_X);
//        assert_eq!(cpu.reg_a, 0xB6);
//    }
//    #[test]
//    fn ora_abs_y() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x19, AddressingMode::ABS_Y);
//        assert_eq!(cpu.reg_a, 0xB6);
//    }
//    #[test]
//    fn ora_zp() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x05, AddressingMode::ZP);
//        assert_eq!(cpu.reg_a, 0xB6);
//    }
//    #[test]
//    fn ora_zp_x() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x15, AddressingMode::ZP_X);
//        assert_eq!(cpu.reg_a, 0xB6);
//    }
//    #[test]
//    fn ora_ind_x() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x01, AddressingMode::IND_X);
//        assert_eq!(cpu.reg_a, 0xB6);
//    }
//    #[test]
//    fn ora_ind_y() {
//        let cpu = test_instruction(InstructionType::LOGIC, 0x11, AddressingMode::IND_Y);
//        assert_eq!(cpu.reg_a, 0xB6);
//    }
//}
