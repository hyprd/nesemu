use super::Mapper;

pub struct NROM {}

impl NROM {
    pub fn new() -> Self {
        NROM {

        }
    }
}

impl Mapper for NROM {
    fn map_prg(&self, address: u16) -> u16 {
        address - 0x8000
    }
    fn map_chr(&self, address: u16) -> u16 {
        address
    } 
}
