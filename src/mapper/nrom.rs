use super::Mapper;

pub struct NROM {}

impl NROM {
    pub fn new() -> Self {
        NROM {}
    }
}

impl Mapper for NROM {
    fn map_prg(&self, address: u16) -> u32 {
        (address - 0x8000) as u32
    }
    fn map_chr(&self, address: u16) -> u32 {
        address as u32
    }
    fn bank_select(&mut self, value: u8) {}
}
