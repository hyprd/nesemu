const WIDTH: usize = 256;
const HEIGHT: usize = 240;

#[rustfmt::skip]
pub static PALETTE: [(u8, u8, u8); 64] = [
        (0x80, 0x80, 0x80), (0x00, 0x3D, 0xA6), (0x00, 0x12, 0xB0), (0x44, 0x00, 0x96), (0xA1, 0x00, 0x5E),
        (0xC7, 0x00, 0x28), (0xBA, 0x06, 0x00), (0x8C, 0x17, 0x00), (0x5C, 0x2F, 0x00), (0x10, 0x45, 0x00),
        (0x05, 0x4A, 0x00), (0x00, 0x47, 0x2E), (0x00, 0x41, 0x66), (0x00, 0x00, 0x00), (0x05, 0x05, 0x05),
        (0x05, 0x05, 0x05), (0xC7, 0xC7, 0xC7), (0x00, 0x77, 0xFF), (0x21, 0x55, 0xFF), (0x82, 0x37, 0xFA),
        (0xEB, 0x2F, 0xB5), (0xFF, 0x29, 0x50), (0xFF, 0x22, 0x00), (0xD6, 0x32, 0x00), (0xC4, 0x62, 0x00),
        (0x35, 0x80, 0x00), (0x05, 0x8F, 0x00), (0x00, 0x8A, 0x55), (0x00, 0x99, 0xCC), (0x21, 0x21, 0x21),
        (0x09, 0x09, 0x09), (0x09, 0x09, 0x09), (0xFF, 0xFF, 0xFF), (0x0F, 0xD7, 0xFF), (0x69, 0xA2, 0xFF),
        (0xD4, 0x80, 0xFF), (0xFF, 0x45, 0xF3), (0xFF, 0x61, 0x8B), (0xFF, 0x88, 0x33), (0xFF, 0x9C, 0x12),
        (0xFA, 0xBC, 0x20), (0x9F, 0xE3, 0x0E), (0x2B, 0xF0, 0x35), (0x0C, 0xF0, 0xA4), (0x05, 0xFB, 0xFF),
        (0x5E, 0x5E, 0x5E), (0x0D, 0x0D, 0x0D), (0x0D, 0x0D, 0x0D), (0xFF, 0xFF, 0xFF), (0xA6, 0xFC, 0xFF),
        (0xB3, 0xEC, 0xFF), (0xDA, 0xAB, 0xEB), (0xFF, 0xA8, 0xF9), (0xFF, 0xAB, 0xB3), (0xFF, 0xD2, 0xB0),
        (0xFF, 0xEF, 0xA6), (0xFF, 0xF7, 0x9C), (0xD7, 0xE8, 0x95), (0xA6, 0xED, 0xAF), (0xA2, 0xF2, 0xDA),
        (0x99, 0xFF, 0xFC), (0xDD, 0xDD, 0xDD), (0x11, 0x11, 0x11), (0x11, 0x11, 0x11)
];

pub struct Frame {
    pub frame_data: Vec<u8>,
}

impl Frame {
    pub fn new() -> Self {
        Frame {
            frame_data: vec![0; WIDTH * HEIGHT * 3],
        }
    }

    pub fn set_pixel(&mut self, x_pos: usize, y_pos: usize, colour: (u8, u8, u8)) {
        let base = y_pos * 3 * WIDTH + x_pos * 3;
        if base + 2 < self.frame_data.len() {
            self.frame_data[base] = colour.0;
            self.frame_data[base + 1] = colour.1;
            self.frame_data[base + 2] = colour.2;
        }
    }

    pub fn show_tile_bank(chr_rom: &Vec<u8>, bank: usize) -> Frame {
        if bank > 1 {
            panic!("Tile bank choice greater than 1");
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
                        0b00 => PALETTE[0x01],
                        0b01 => PALETTE[0x23],
                        0b10 => PALETTE[0x27],
                        0b11 => PALETTE[0x30],
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
