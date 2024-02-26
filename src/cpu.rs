struct CPU {
    reg_a: u8,
    reg_pc: u8,
}

impl CPU {
    fn new() -> Self {
        CPU {
            reg_a: 0,
            reg_pc: 0,
        }
    }

    fn decode(&mut self, memory: Vec<u8>) {
        self.reg_pc = 0;
        loop {
            let opcode = memory[self.reg_pc as usize];
            self.reg_pc += 1;
            match opcode {
                0x00 => {
                    return;
                }
                0x01..=0xA8 => {
                    return;
                }
                0xA9 => {
                    let immediate = memory[self.reg_pc as usize];
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
        let memory = vec![0xA9, 0x05, 0x00];
        let mut cpu = CPU::new();
        cpu.decode(memory);
        assert_eq!(cpu.reg_a, 0x05);
    }
}
