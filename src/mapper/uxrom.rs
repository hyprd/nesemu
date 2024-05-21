use super::Mapper;

pub struct UXROM {
    bank_select_register: u8,
    prg_banks: u8, 
}

impl UXROM {
    pub fn new(banks: u8) -> Self {
        UXROM {
            bank_select_register: 0x00,
            prg_banks: banks,
        }
    }
}

impl Mapper for UXROM {
    /*
    *   0x8000 - 0xBFFF = Switchable PRG ROM 
    *   0xC000 - 0xFFFF = Fixed PRG ROM
    */

    fn map_prg(&self, address: u16) -> u32 {
        let bank = match address <= 0xC000 { 
            // If address is in switchable bank address space...
            true => self.bank_select_register,
            // Since 0xC000-0xFFFF is fixed to the last bank, need to sub one.
            false => self.prg_banks - 1, 
        } as u32;
        let mapped_address = (address & 0x3FFF) as u32;
        (0x4000 * bank + mapped_address) as u32
    }

    fn map_chr(&self, address: u16) -> u32 {
        address as u32
    }

    fn bank_select(&mut self, value: u8) {
        self.bank_select_register = (value & 0x0F);
    }
}
