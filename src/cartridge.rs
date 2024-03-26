pub enum MirroringType {
    Vertical,
    Horizontal,
    FourScreen,
}

pub struct ROM {
    pub rom_prg: Vec<u8>,
    pub rom_chr: Vec<u8>,
    pub rom_mapper: u8,
    pub mirroring_type: MirroringType,   
}

impl ROM {
    pub fn new(binary: &Vec<u8>) -> Result<ROM, String> {
    }
}
