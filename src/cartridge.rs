const NES_IDENTIFIER_TAG: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];
const ROM_BANK_SIZE: usize = 16384;
const VROM_BANK_SIZE: usize = 8192;

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
    pub fn new(binary: &[u8]) -> Result<ROM, String> {
        // Define location of control bytes.
        let control_byte_one = binary[6];
        let control_byte_two = binary[7];
        
        // Get cartridge mapper.
        let mapper = control_byte_two & 0b11110000 | control_byte_one >> 4;

        // Determine whether binary is identifiable as an NES cartridge.
        if (binary[0..4]) != NES_IDENTIFIER_TAG {
            return Err("File is in incorrect format".to_string());
        }
        
        // Get the count of 8KB ROM and 16KB VROM banks. Multiply by 
        // page size to get total size.
        let rom_prg_size = binary[4] as usize * ROM_BANK_SIZE;
        let rom_chr_size = binary[5] as usize * VROM_BANK_SIZE;

        // Evaluate whether trainer data is used.
        let is_trainer = (control_byte_one & 0b100) != 0;
        
        // Lower bounds of addressable space for both PRG and CHR ROM.
        let rom_prg_start = 16 + if is_trainer { 512 } else { 0 };
        let rom_chr_start = rom_prg_start + rom_prg_size;

        // Determine the mirroring type of the cartridge
        let mirroring_plane = (control_byte_one & 0b1) != 0;
        let mirroring_four_screen = (control_byte_one & 0b1000) != 0;
        let mirroring = match (mirroring_four_screen, mirroring_plane) {
            (true, _) => MirroringType::FourScreen,
            (false, true) => MirroringType::Vertical,
            (false, false) => MirroringType::Horizontal,
        };

        Ok(ROM {
            rom_prg: binary[rom_prg_start..(rom_prg_start + rom_prg_size)].to_vec(),
            rom_chr: binary[rom_chr_start..(rom_chr_start + rom_chr_size)].to_vec(),
            rom_mapper: mapper,
            mirroring_type: mirroring,
        })
    }
}
