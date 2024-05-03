use crate::ppu::PPU;
use rand::Rng;

const WIDTH: usize = 256;
const HEIGHT: usize = 240;

pub struct Frame {
    pub frame_data: Vec<u8>,
}

struct Rectangle {
    x_1: usize,
    y_1: usize,
    x_2: usize,
    y_2: usize,
}

impl Rectangle {
    fn new(x_1: usize, y_1: usize, x_2: usize, y_2: usize) -> Self {
        Rectangle { x_1, y_1, x_2, y_2 }
    }
}

impl Frame {
    pub fn new() -> Self {
        Frame {
            frame_data: vec![0; WIDTH * HEIGHT * 3],
        }
    }

    pub fn read_palette_from_file(file_name: &str) -> Vec<(u8, u8, u8)> {
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

    fn get_background_palette(ppu: &PPU, row: usize, col: usize) -> Vec<u8> {
        // https://www.nesdev.org/wiki/PPU_attribute_tables
        let attribute_table_index = row / 4 * 8 + col / 4;
        // Colour palette that exists in nametable..
        let attribute_byte = ppu.vram[960 + attribute_table_index];
        let palette_index = match (col % 4 / 2, row % 4 / 2) {
            (0, 0) => attribute_byte & 0b11,
            (1, 0) => (attribute_byte >> 2) & 0b11,
            (0, 1) => (attribute_byte >> 4) & 0b11,
            (1, 1) => (attribute_byte >> 6) & 0b11,
            (_, _) => panic!("Invalid index value fetching background palette"),
        };
        let start: usize = 1 + (palette_index as usize) * 4;
        vec![
            ppu.palette_table[0],
            ppu.palette_table[start],
            ppu.palette_table[start + 1],
            ppu.palette_table[start + 2],
        ]
    }

    fn get_sprite_palette(ppu: &PPU, palette_index: u8) -> Vec<u8> {
        let start = (palette_index * 4 + 0x11) as usize;
        vec![
            0,
            ppu.palette_table[start],
            ppu.palette_table[start + 1],
            ppu.palette_table[start + 2],
        ]
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
        let bank = ppu.reg_controller.background_pattern_table_address();
        for i in 0..0x3C0 {
            let tile = ppu.vram[i] as u16;
            let col = i % 32;
            let row = i / 32;
            let tile_data =
                &ppu.chr_rom[(bank + tile * 16) as usize..=(bank + tile * 16 + 15) as usize];
            let background_palette = Self::get_background_palette(ppu, row, col);
            for y in 0..=7 {
                let mut hh = tile_data[y];
                let mut ll = tile_data[y + 8];
                for x in (0..=7).rev() {
                    let value = (1 & ll) << 1 | (0x01 & hh);
                    ll >>= 1;
                    hh >>= 1;
                    let colour = match value {
                        0b00 => palette[ppu.palette_table[0] as usize],
                        0b01 => palette[background_palette[1] as usize],
                        0b10 => palette[background_palette[2] as usize],
                        0b11 => palette[background_palette[3] as usize],
                        _ => panic!("Couldn't set BG palette colour"),
                    };
                    frame.set_pixel(col * 8 + x, row * 8 + y, colour);
                }
            }
        }

        // // Iterate throguh OAM data
        for j in (0..256).step_by(4).rev() {
            /*
             * BYTE 0 - Y POSITION TOP
             * BYTE 1 = TILE INDEX
             * BYTE 2 - ATTRIBUTES
             * BYTE 3 - X POSITION LEFT
             */
            let tile_y = ppu.oam_data[j];
            let tile_index = ppu.oam_data[j + 1] as u16;
            let attributes = ppu.oam_data[j + 2];
            let tile_x = ppu.oam_data[j + 3] as usize;
            let palette_index = attributes & 0b11;
            let sprite_palette = Self::get_sprite_palette(ppu, palette_index);
            let bank: u16 = ppu.reg_controller.sprite_pattern_table_address();
            let tile = &ppu.chr_rom
                [(bank + tile_index * 16) as usize..=(bank + tile_index * 16 + 15) as usize];
            for y in 0..=7 {
                let mut hh = tile[y];
                let mut ll = tile[y + 8];
                'k: for x in (0..=7).rev() {
                    let value = (0x01 & hh) << 1 | (0x01 & ll);
                    hh >>= 1;
                    ll >>= 1;
                    // Assign colour of a given pixel
                    let colour = match value {
                        0b00 => continue 'k,
                        0b01 => palette[sprite_palette[1] as usize],
                        0b10 => palette[sprite_palette[2] as usize],
                        0b11 => palette[sprite_palette[3] as usize],
                        _ => panic!("Illegal palette value"),
                    };
                    // flip horizontal, flip vertical
                    match (attributes >> 6 & 0x01, attributes >> 7 & 0x01) {
                        (0, 0) => frame.set_pixel(
                            (tile_x.wrapping_add(x)) as usize,
                            (tile_y.wrapping_add(y as u8) as usize),
                            colour,
                        ),
                        (1, 0) => frame.set_pixel(
                            (tile_x.wrapping_add(7).wrapping_sub(x)) as usize,
                            (tile_y.wrapping_add(y as u8)) as usize,
                            colour,
                        ),
                        (0, 1) => frame.set_pixel(
                            (tile_x.wrapping_add(x)) as usize,
                            (tile_y.wrapping_add(7).wrapping_sub(y as u8)) as usize,
                            colour,
                        ),
                        (1, 1) => frame.set_pixel(
                            (tile_x.wrapping_add(7).wrapping_sub(x)) as usize,
                            (tile_y.wrapping_add(7).wrapping_sub(y as u8)) as usize,
                            colour,
                        ),
                        (_, _) => {}
                    }
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
