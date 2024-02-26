struct CPU {
    reg_a: u8,
    reg_x: u8,
    reg_y: u8,
    reg_pc: u16,
    memory: [u8; 0xFFFF],
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
enum AddressingMode {
    IMM,
    ZP,
    ZP_X,
    ZP_Y,
    ABS,
    ABS_X,
    ABS_Y,
    IND_X,
    IND_Y,
    NONE,
}

impl CPU {
    fn new() -> Self {
        CPU {
            reg_a: 0,
            reg_x: 0,
            reg_y: 0,
            reg_pc: 0,
            memory: [0; 0xFFFF],
        }
    }

    fn resolve_addressing_mode(&mut self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::IMM => self.reg_pc,

            AddressingMode::ZP => self.mem_read(self.reg_pc) as u16,

            AddressingMode::ZP_X => {
                let base_address = self.mem_read(self.reg_pc);
                let effective_address = base_address.wrapping_add(self.reg_x) as u16;
                effective_address
            }

            AddressingMode::ZP_Y => {
                let base_address = self.mem_read(self.reg_pc);
                let effective_address = base_address.wrapping_add(self.reg_y) as u16;
                effective_address
            }

            AddressingMode::ABS => self.mem_read_u16(self.reg_pc),

            AddressingMode::ABS_X => {
                let base_address = self.mem_read_u16(self.reg_pc);
                let effective_address = base_address.wrapping_add(self.reg_x as u16);
                effective_address
            }

            AddressingMode::ABS_Y => {
                let base_address = self.mem_read_u16(self.reg_pc);
                let effective_address = base_address.wrapping_add(self.reg_y as u16);
                effective_address
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
                let hh = self.mem_read((base_address as u8).wrapping_add(1) as u16);
                let llhh = (hh as u16) << 8 | (ll as u16);
                let effective_address = llhh.wrapping_add(self.reg_y as u16);
                effective_address
            }

            AddressingMode::NONE => {
                panic!("Addressing mode {:?} not supported", mode);
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
        (hh << 8) | (ll as u16)
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
        loop {
            let opcode = self.mem_read(self.reg_pc);
            self.reg_pc += 1;
            match opcode {
                0x00 => {
                    return;
                }
                0x01..=0xA8 => {
                    return;
                }
                0xA9 => {
                    let immediate = self.memory[self.reg_pc as usize];
                    self.reg_pc += 1;
                    self.lda(immediate);
                }
                0xAA..=u8::MAX => {}
            }
        }
    }
    fn lda(&mut self, value: u8) {
        self.reg_a = value;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn lda_imm() {
        let mut cpu = CPU::new();
        let cart = vec![0xA9, 0x05, 0x00];
        cpu.mem_run_prg(cart);
        cpu.execute();
        assert_eq!(cpu.reg_a, 0x05);
    }
}
