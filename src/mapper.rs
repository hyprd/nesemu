pub mod nrom;
pub mod uxrom;

pub trait Mapper {
    fn map_prg(&self, address: u16) -> u32;
    fn map_chr(&self, address: u16) -> u32;
    fn bank_select(&mut self, value: u8);
}
