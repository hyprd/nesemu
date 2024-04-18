use crate::cartridge::MirroringType;
use reg_addr::PPUADDR;
use reg_controller::PPUCTRL;
use reg_mask::PPUMASK;
use reg_scroll::PPUSCROLL;
use reg_status::PPUSTATUS;

pub mod reg_addr;
pub mod reg_controller;
pub mod reg_mask;
pub mod reg_scroll;
pub mod reg_status;

pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette_table: [u8; 32],
    pub vram: [u8; 2048],
    pub mirroring: MirroringType,
    pub reg_v: u16,
    pub reg_t: u16,
    pub reg_x: u8,
    pub reg_w: bool,
    pub reg_address: PPUADDR,
    pub reg_controller: PPUCTRL,
    pub reg_mask: PPUMASK,
    pub reg_status: PPUSTATUS,
    pub reg_scroll: PPUSCROLL,
    internal_data_buffer: u8,
    
    pub oam_data: [u8; 256],
    pub oam_address: u8,
}

impl PPU {
    /*
     * Accessing PPU memory
     *   1. CPU writes the address it wants to access in PPU memory space by writing 2 bytes to 0x2006.
     *   2. PPU accesses data stored at this address and stores it in its own internal buffer.
     *   3. PPU increments the address register, determined by the state of the control register
     *      @ 0x2000.
     *   4. CPU reads data register 0x2007, prompting the PPU to return the internal buffer data.
     */
    pub fn new(chr_rom: Vec<u8>, mirroring: MirroringType) -> Self {
        PPU {
            chr_rom,
            palette_table: [0; 32],
            vram: [0; 2048],
            mirroring,
            reg_v: 0,
            reg_t: 0,
            reg_x: 0,
            reg_w: true,
            reg_address: PPUADDR::new(),
            reg_controller: PPUCTRL::new(),
            reg_status: PPUSTATUS::new(),
            reg_mask: PPUMASK::new(),
            reg_scroll: PPUSCROLL::new(),
            internal_data_buffer: 0,
            oam_data: [0; 256],
            oam_address: 0,
        }
    }

    pub fn write_to_oam_address(&mut self, value : u8) {
        self.oam_address = value;
    }

    pub fn write_to_oam_data(&mut self, value: u8) {
        self.oam_data[self.oam_address as usize] = value;
        self.oam_address = self.oam_address.wrapping_add(1);
    }

    pub fn write_to_oam_dma(&mut self, data: &[u8; 0xFF]) {
        for x in data.iter() {
            self.oam_data[self.oam_address as usize] = *x;
            self.oam_address = self.oam_address.wrapping_add(1);
        }
    }

    pub fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_address as usize]
    }
    
    pub fn write_to_reg_addr(&mut self, value: u8) {
        self.reg_address.update(value);
    }
    
    pub fn write_to_reg_ctrl(&mut self, value: u8) {
        self.reg_controller.update(value);
    }
    
    pub fn write_to_reg_mask(&mut self, value: u8) {
        self.reg_mask.update(value);
    }
    
    pub fn write_to_reg_scroll(&mut self, value: u8) {
        self.reg_scroll.write_scroll(value);
    }
    
    pub fn read_status(&mut self, value: u8) -> u8 {
        let current_status = self.reg_status.bits();
        // VBLANK is cleared after reading 0x2002
        self.reg_status.reset_vblank();
        current_status
    }

    pub fn increment_vram_address(&mut self) {
        if self.reg_controller.contains(PPUCTRL::VRAM_ADDR_INCREMENT) {
            self.reg_address.increment(1);
        } else {
            self.reg_address.increment(32);
        }
    }

    pub fn read_data(&mut self) -> u8 {
        let address = self.reg_address.get();
        self.increment_vram_address();
        match address {
            0..=0x1FFF => {
                let buffer_data = self.internal_data_buffer;
                self.internal_data_buffer = self.chr_rom[address as usize];
                buffer_data
            }
            0x2000..=0x2FFF => {
                let buffer_data = self.internal_data_buffer;
                self.internal_data_buffer = self.vram[self.mirror_vram(address) as usize];
                buffer_data
            }
            0x3000..=0x3EFF => panic!("Illegal memory space access at {}", address),
            0x3F00..=0x3FFF => todo!("Palette table"),
            _ => panic!("Illegal access of mirrored space = {}", address),
        }
    }

    pub fn mirror_vram(&self, address: u16) -> u16 {
        // Mirror down to addressable VRAM space
        let mirror_down = address & 0x2FFF;
        // Get position that address exists within VRAM space
        let vram_position = mirror_down - 0x2000;
        // Get corresponding nametable of given address
        let nametable = vram_position / 0x400;

        match (&self.mirroring, nametable) {
            (MirroringType::Vertical, 2)
            | (MirroringType::Vertical, 3)
            | (MirroringType::Horizontal, 3) => vram_position - 0x800,
            (MirroringType::Horizontal, 1) | (MirroringType::Horizontal, 2) => {
                vram_position - 0x400
            }
            _ => vram_position,
        }
    }
}
