use super::Mapper;

pub struct UXROM {
    bank_select_register: u8,
}

impl UXROM {
    pub fn new() -> Self {
        UXROM {
            bank_select_register: 0x00,
        }
    }
}

impl Mapper for UXROM {
    /*
    *   0x8000 - 0xBFFF = Switchable PRG ROM 
    *   0xC000 - 0xFFFF = Fixed PRG ROM
    */

    fn map_prg(&self, address: u16) -> u16 {
        let bank = match address <= 0xC000 { 
            // If address is in switchable bank address space...
            true => self.bank_select_register,
            false => 0,
        } as u16;
        let mapped_address = address & 0x3FFF;
        0x4000 * bank + mapped_address
    }

    fn map_chr(&self, address: u16) -> u16 {
        address
    }

    fn bank_select(&mut self, value: u8) {
        self.bank_select_register = (value & 0x0F);
    }
}