pub mod nrom;

pub trait Mapper {
    fn map_prg(&self, address: u16) -> u16;
    fn map_chr(&self, address: u16) -> u16;
}
