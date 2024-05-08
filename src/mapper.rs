use crate::cartridge::MirroringType;

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
    pub fn new(map_type: MapperType, mirr_type: MirroringType) -> Self {
        Mapper {
            mapper_type: map_type,
            chr_ram: vec![],
            chr_banks: 0,
            prg_rom: vec![],
            prg_banks: 0,
            mirroring_type: mirr_type,
        }
    }
    pub fn mapper_write(value: u8) {
        
    }

    pub fn mapper_read() -> u8 {
        0
    }
}
