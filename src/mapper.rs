use crate::cartridge::MirroringType;

mod nrom;

pub enum MapperType {
    NROM,
    UXROM,
}

pub struct Mapper {
    mapper_type: MapperType,
    chr_ram: Vec<u8>,
    chr_banks: u8,
    prg_rom: Vec<u8>,
    prg_banks: u8,
    mirroring_type: MirroringType,
}

impl Mapper {
    pub fn new(mapper_type: MapperType, mirr_type: MirroringType) -> Mapper {
        match mapper_type {
            MapperType::NROM => nrom::NROM::load(mirr_type),
            MapperType::UXROM => todo!(),
        }
    }
}
