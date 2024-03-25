#![allow(warnings)]

pub struct Bus {
    vram: [u8; 0x800]
}

impl Bus {
    pub fn new() -> Self {
        Bus {
            vram: [0; 0x800]
        }
    }
}


