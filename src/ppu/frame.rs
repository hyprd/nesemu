use rand::Rng;
use crate::ppu::PPU;

const WIDTH: usize = 256;
const HEIGHT: usize = 240;
const BACKGROUND_TILE_MAX: u16 = 960;

pub struct Frame {
    pub frame_data: Vec<u8>,
}

impl Frame {
    pub fn new() -> Self {
        Frame {
            frame_data: vec![0; WIDTH * HEIGHT * 3],
        }
    }

    pub fn read_palette_from_file(file_name: &str) -> Vec<(u8,u8,u8)> {
        let mut palette_vec: Vec<(u8, u8, u8)> = vec![];
        for line in std::fs::read_to_string(file_name).unwrap().lines() {
            let palette_one = u8::from_str_radix(&line[0..2], 16).unwrap();
            let palette_two = u8::from_str_radix(&line[2..4], 16).unwrap();
            let palette_three = u8::from_str_radix(&line[4..6], 16).unwrap();
            palette_vec.push(((palette_one), (palette_two), (palette_three)));
        }
        if palette_vec.len() > 64 {
            panic!("Palette file is too big");
        }
        palette_vec
    }

    pub fn set_pixel(&mut self, x_pos: usize, y_pos: usize, colour: (u8, u8, u8)) {
        let base = y_pos * 3 * WIDTH + x_pos * 3;
        if base + 2 < self.frame_data.len() {
            self.frame_data[base] = colour.0;
            self.frame_data[base + 1] = colour.1;
            self.frame_data[base + 2] = colour.2;
        }
    }

    pub fn render(ppu: &PPU, frame: &mut Frame, palette: Vec<(u8, u8, u8)>) {
    let bg_tbl_address = ppu.reg_controller.background_pattern_table_address() as u16;
    for i in 0..BACKGROUND_TILE_MAX {
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
    pub fn show_tile_bank(palette: Vec<(u8, u8, u8)>, chr_rom: &Vec<u8>, bank: usize) -> Frame {
        if bank > 1 {
            panic!("Tile bank choice greater than 1");
        }
        
        let mut rng = rand::thread_rng();
        let mut palette_indexes: Vec<usize> = vec![];
        for p in 0..4 {
            palette_indexes.push(rng.gen_range(0..55));
        }

        let mut frame = Frame::new();
        let mut tile_y = 0;
        let mut tile_x = 0;
        let tile_bank = (bank * 0x1000) as usize;
        // Render limit for tiles on each row
        let tile_limit_per_row = WIDTH / 8;
        // Iterate over tiles in bank (256 total per bank)
        for tile_n in 0..255 {
            // Go to next row
            if tile_n != 0 && tile_n % tile_limit_per_row == 0 {
                tile_y += 8;
                tile_x = 0;
            }
            // Get data for tile at given address
            let tile = &chr_rom[(tile_bank + tile_n * 16)..=(tile_bank + tile_n * 16 + 15)];
            // Go pixel by pixel..
            for y in 0..=7 {
                // A tile is represented with 16 bits (or two bytes). To get the colour value of a
                // given pixel, you must combine the bit position in byte one with the same bit
                // position in byte two.
                let mut hh = tile[y];
                let mut ll = tile[y + 8];

                for x in (0..=7).rev() {
                    // Combine the bits together (0b00, 0b01, 0b10 or 0b11)
                    let value = (0x01 & hh) << 1 | (0x01 & ll);
                    hh >>= 1;
                    ll >>= 1;
                    // Assign colour of a given pixel
                    let colour = match value {
                        0b00 => palette[palette_indexes[0]],
                        0b01 => palette[palette_indexes[1]],
                        0b10 => palette[palette_indexes[2]],
                        0b11 => palette[palette_indexes[3]],
                        _ => panic!("Illegal palette value"),
                    };
                    frame.set_pixel(tile_x + x, tile_y + y, colour)
                }
            }
            // Move pos to render next tile on the row
            tile_x += 8;
        }
        frame
    }
}
