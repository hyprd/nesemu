use std::ops::{BitAnd, BitAndAssign};

use bitflags::Flags;
// 7  bit  0
// ---- ----
// BGRs bMmG
// |||| ||||
// |||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
// |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
// |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
// |||| +---- 1: Show background
// |||+------ 1: Show sprites
// ||+------- Emphasize red (green on PAL/Dendy)
// |+-------- Emphasize green (red on PAL/Dendy)
// +--------- Emphasize blue
bitflags! {
    pub struct PPUMASK : u8 {
        const GREYSCALE = 0b000000001;
        const SHOW_BACKGROUND_LEFTMOST = 0b00000010;
        const SHOW_SPRITES_LEFTMOST = 0b00000100;
        const SHOW_BACKGROUND = 0b00001000;
        const SHOW_SPRITES = 0b00010000;
        const EMPHASIZE_RED = 0b00100000;
        const EMPHASIZE_GREEN = 0b01000000;
        const EMPHASIZE_BLUE = 0b10000000;
    }
}

pub enum Colour {
    RED,
    GREEN,
    BLUE,
}

impl PPUMASK {
    pub fn new() -> Self {
        PPUMASK::from_bits_truncate(0b000000000)
    }
    pub fn is_greyscale_enabled(&self) -> bool {
        self.contains(PPUMASK::GREYSCALE)        
    }

    pub fn is_background_leftmost_enabled(&self) -> bool {
        self.contains(PPUMASK::SHOW_BACKGROUND_LEFTMOST)
    }

    pub fn is_sprite_leftmost_enabled(&self) -> bool {
        self.contains(PPUMASK::SHOW_SPRITES_LEFTMOST)
    }

    pub fn is_background_enabled(&self) -> bool {
        self.contains(PPUMASK::SHOW_BACKGROUND)
    }

    pub fn is_sprite_enabled(&self) -> bool {
        self.contains(PPUMASK::SHOW_SPRITES)
    }
    // https://www.nesdev.org/wiki/Colour_emphasis
    pub fn emphasize(&self) -> Vec<Colour> {
        let mut colours = vec![];
        if self.contains(PPUMASK::EMPHASIZE_RED) {
            colours.push(Colour::RED);
        }
        if self.contains(PPUMASK::EMPHASIZE_BLUE) {
            colours.push(Colour::BLUE);
        }
        if self.contains(PPUMASK::EMPHASIZE_GREEN) {
            colours.push(Colour::GREEN);
        }
        colours
    }

    pub fn update(&mut self, value: u8) {
        *self = PPUMASK::from_bits_truncate(value);
    }
}
