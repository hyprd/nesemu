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

const STACK_HEAD: u16 = 0x0100;

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
        self.reg_status =  StatusFlags::from_bits_truncate(0b100100);
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

    fn execute(&mut self) {
        let ref jmp_table: HashMap<u8, &'static opcodes::Opcode> = *opcodes::OPCODES_JMP_TABLE;
        loop {
            let opcode = self.mem_read(self.reg_pc);
            self.reg_pc += 1;
            let instruction = jmp_table.get(&opcode);
            let pc_snapshot = self.reg_pc;
            match opcode {
                0xA9 | 0xAD | 0xBD | 0xB9 | 0xA5 | 0xB5 | 0xA1 | 0xB1 => {
                    self.lda(&instruction.unwrap().addressing_mode)
                }
                0xA2 | 0xAE | 0xBE | 0xA6 | 0xB6 => self.ldx(&instruction.unwrap().addressing_mode),
                0xA0 | 0xAC | 0xBC | 0xA4 | 0xB4 => self.ldy(&instruction.unwrap().addressing_mode),
                0x8D | 0x9D | 0x99 | 0x85 | 0x95 | 0x81 | 0x91 => {
                    self.sta(&instruction.unwrap().addressing_mode)
                }
                0x8E | 0x86 | 0x96 => self.stx(&instruction.unwrap().addressing_mode),
                0x8C | 0x84 | 0x94 => self.sty(&instruction.unwrap().addressing_mode),
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
                0x0A | 0x0E | 0x1E | 0x06 | 0x16 => self.asl(&instruction.unwrap().addressing_mode),
                0x4A | 0x4E | 0x5E | 0x46 | 0x56 => self.lsr(&instruction.unwrap().addressing_mode),
                0x2A | 0x2E | 0x3E | 0x26 | 0x36 => self.rol(&instruction.unwrap().addressing_mode),
                0x6A | 0x6E | 0x7E | 0x66 | 0x76 => self.ror(&instruction.unwrap().addressing_mode),
                0x29 | 0x2D | 0x3D | 0x39 | 0x25 | 0x35 | 0x21 | 0x31 => {
                    self.and(&instruction.unwrap().addressing_mode)
                }
                0x2C | 0x24 => self.bit(&instruction.unwrap().addressing_mode),
                0x49 | 0x4D | 0x5D | 0x59 | 0x45 | 0x55 | 0x41 | 0x51 => {
                    self.eor(&instruction.unwrap().addressing_mode)
                }
                0x09 | 0x0D | 0x1D | 0x19 | 0x05 | 0x15 | 0x01 | 0x11 => {
                    self.ora(&instruction.unwrap().addressing_mode)
                }
                0x69 | 0x6D | 0x7D | 0x79 | 0x65 | 0x75 | 0x61 | 0x71 => {
                    self.adc(&instruction.unwrap().addressing_mode)
                }
                0xC9 | 0xCD | 0xDD | 0xD9 | 0xC5 | 0xD5 | 0xC1 | 0xD1 => {
                    self.cmp(&instruction.unwrap().addressing_mode)
                }
                0xE0 | 0xEC | 0xE4 => self.cpx(&instruction.unwrap().addressing_mode),
                0xC0 | 0xCC | 0xC4 => self.cpy(&instruction.unwrap().addressing_mode),
                0xE9 | 0xED | 0xFD | 0xF9 | 0xE5 | 0xF5 | 0xE1 | 0xF1 => {
                    self.sbc(&instruction.unwrap().addressing_mode)
                }
                0xCE | 0xDE | 0xC6 | 0xD6 => self.dec(&instruction.unwrap().addressing_mode),
                0xCA => self.dex(),
                0x88 => self.dey(),
                0xEE | 0xFE | 0xE6 | 0xF6 => self.inc(&instruction.unwrap().addressing_mode),
                0xE8 => self.inx(),
                0xC8 => self.iny(),
                0x00 => self.brk(),
                0x4C | 0x6C => self.jmp(&instruction.unwrap().addressing_mode),
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
                self.reg_pc += (&instruction.unwrap().length - 1) as u16;
            }
        }
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let address = self.resolve_addressing_mode(mode);
        let value = self.mem_read(address);
        self.reg_a = value;
    }

    fn ldx(&mut self, mode: &AddressingMode) {

    }
    fn ldy(&mut self, mode: &AddressingMode) {}
    fn sta(&mut self, mode: &AddressingMode) {}
    fn stx(&mut self, mode: &AddressingMode) {}
    fn sty(&mut self, mode: &AddressingMode) {}
    fn tax(&mut self) {}
    fn tay(&mut self) {}
    fn tsx(&mut self) {}
    fn txa(&mut self) {}
    fn txs(&mut self) {}
    fn tya(&mut self) {}
    fn pha(&mut self) {}
    fn php(&mut self) {}
    fn pla(&mut self) {}
    fn plp(&mut self) {}
    fn asl(&mut self, mode: &AddressingMode) {}
    fn lsr(&mut self, mode: &AddressingMode) {}
    fn rol(&mut self, mode: &AddressingMode) {}
    fn ror(&mut self, mode: &AddressingMode) {}
    fn and(&mut self, mode: &AddressingMode) {}
    fn bit(&mut self, mode: &AddressingMode) {}
    fn eor(&mut self, mode: &AddressingMode) {}
    fn ora(&mut self, mode: &AddressingMode) {}
    fn adc(&mut self, mode: &AddressingMode) {}
    fn cmp(&mut self, mode: &AddressingMode) {}
    fn cpx(&mut self, mode: &AddressingMode) {}
    fn cpy(&mut self, mode: &AddressingMode) {}
    fn sbc(&mut self, mode: &AddressingMode) {}
    fn dec(&mut self, mode: &AddressingMode) {}
    fn dex(&mut self) {}
    fn dey(&mut self) {}
    fn inc(&mut self, mode: &AddressingMode) {}
    fn inx(&mut self) {}
    fn iny(&mut self) {}
    fn brk(&mut self) {}
    fn jmp(&mut self, mode: &AddressingMode) {}
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
        //cpu.execute();
        assert_eq!(cpu.reg_a, 0x05);
    }
}
