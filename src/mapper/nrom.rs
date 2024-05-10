use super::{Mapper, MapperType};
use crate::cartridge::MirroringType;

pub struct NROM {}

impl NROM {
    pub fn load(mirr_type: MirroringType) -> Mapper {
        Mapper {
            mapper_type: MapperType::NROM,
            chr_banks: 0,
            chr_ram: vec![],
            mirroring_type: mirr_type,
            prg_banks: 0,
            prg_rom: vec![],
        }
    }

}
