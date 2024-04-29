pub mod frame;

use crate::ppu::PPU;
use crate::Frame;

const BG_TILE_MAX: u16 = 960;

pub fn render(ppu: &PPU, frame: &mut Frame, palette: Vec<(u8, u8, u8)>) {
    let bg_tbl_address = ppu.reg_controller.background_pattern_table_address() as u16;
    for i in 0..BG_TILE_MAX {
        let tile_entry = (bg_tbl_address + ppu.vram[i as usize] as u16) * 16;
        let tile_x = i & 32;
        let tile_y = i / 32;
        let tile_data = &ppu.chr_rom[(tile_entry) as usize..=(tile_entry + 15) as usize];
        for y in 0..7 {
            let mut hh = tile_data[y];
            let mut ll = tile_data[y + 8];
            for x in (0..7).rev() {
                let value = (0x01 & hh) << 1 | (0x01 & ll);
                hh >>= 1;
                ll >>= 1;
                let colour = match value {
                    0b00 => palette[0x01],
                    0b01 => palette[0x23],
                    0b10 => palette[0x26],
                    0b11 => palette[0x30],
                    _ => panic!("Illegal palette value"),
                };
                let xpos = (tile_x * 8 + x) as usize;
                let ypos = (tile_y * 8 + y as u16) as usize;
                frame.set_pixel(xpos, ypos, colour);
            }
        }
    }
}
