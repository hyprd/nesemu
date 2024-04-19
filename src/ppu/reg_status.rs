use bitflags::Flags;
// 7  bit  0
// ---- ----
// VSO. ....
// |||| ||||
// |||+-++++- PPU open bus. Returns stale PPU bus contents.
// ||+------- Sprite overflow. The intent was for this flag to be set
// ||         whenever more than eight sprites appear on a scanline, but a
// ||         hardware bug causes the actual behavior to be more complicated
// ||         and generate false positives as well as false negatives; see
// ||         PPU sprite evaluation. This flag is set during sprite
// ||         evaluation and cleared at dot 1 (the second dot) of the
// ||         pre-render line.
// |+-------- Sprite 0 Hit.  Set when a nonzero pixel of sprite 0 overlaps
// |          a nonzero background pixel; cleared at dot 1 of the pre-render
// |          line.  Used for raster timing.
// +--------- Vertical blank has started (0: not in vblank; 1: in vblank).
//            Set at dot 1 of line 241 (the line *after* the post-render
//            line); cleared after reading $2002 and at dot 1 of the
//            pre-render line.


bitflags! {
    pub struct PPUSTATUS : u8 {
        const BUS_CONTENTS_ONE = 0b00000001;
        const BUS_CONTENTS_TWO = 0b00000010;
        const BUS_CONTENTS_THREE = 0b00000100;
        const BUS_CONTENTS_FOUR = 0b00001000;
        const BUS_CONTENTS_FIVE = 0b00010000;
        const SPRITE_OVERFLOW = 0b00100000;
        const SPRITE_ZERO_HIT = 0b01000000;
        const VBLANK_STARTED = 0b10000000;
    }
}

impl PPUSTATUS {
    pub fn new() -> Self {
        PPUSTATUS::from_bits_truncate(0b0000000)
    }
    pub fn set_sprite_overflow(&mut self, status: bool) {
       self.set(PPUSTATUS::SPRITE_OVERFLOW, status); 
    }
    pub fn set_sprite_zero_hit(&mut self, status: bool) {
       self.set(PPUSTATUS::SPRITE_ZERO_HIT, status); 
    }
    pub fn set_vblank_started(&mut self, status: bool) {
       self.set(PPUSTATUS::VBLANK_STARTED, status); 
    }
    pub fn reset_vblank(&mut self) {
        self.remove(PPUSTATUS::VBLANK_STARTED);
    } 
    pub fn in_vblank(&self) -> bool {
        self.contains(PPUSTATUS::VBLANK_STARTED)
    }
    pub fn get_bits(&self) -> u8 {
        self.bits()
    }
}
