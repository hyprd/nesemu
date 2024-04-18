pub struct PPUSCROLL {
    pub scx: u8,
    pub scy: u8,
    pub write_latch: bool,
}

impl PPUSCROLL {
    pub fn new() -> Self {
        PPUSCROLL {
            scx : 0,
            scy : 0,
            write_latch: false, 
        }
    }
    pub fn write_scroll(&mut self, value: u8) {
        if !self.write_latch {
            self.scx = value;
        } else {
            self.scy = value;
        }
        self.write_latch = !self.write_latch;
    }
    pub fn reset_scroll(&mut self) {
        self.write_latch = false;
    }
}
