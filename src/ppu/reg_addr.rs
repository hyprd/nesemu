pub struct PPUADDR {
    ll: u8,
    hh: u8,
    first_write : bool
}

impl PPUADDR {
    pub fn new() -> Self {
        PPUADDR { 
            ll : 0,
            hh: 0,
            first_write: true,
        }
    }

    fn set(&mut self, value: u16) {
        self.hh = (value >> 8) as u8;
        self.ll = (value & 0xFF) as u8;
    }

    pub fn get(&self) -> u16 {
        ((self.hh as u16) << 8) | self.ll as u16
    }

    pub fn reset(&mut self) {
        self.first_write = true;
    }

    pub fn update(&mut self, data: u8) {
        if self.first_write {
            self.hh = data;
        } else {
            self.ll = data;
        }
        self.first_write = !self.first_write;

        /* 
        * Valid addresses are between 0 - 0x3FFF, so writes above this need 
        * to be mirrored down.
        */
        if self.get() > 0x3FFF {
            self.set(self.get() & 0x3FFF);
        }
    }

    pub fn increment(&mut self, inc: u8) {
        let low = self.ll;
        self.ll = self.ll.wrapping_add(inc);
        if low > self.ll {
            self.hh = self.hh.wrapping_add(1);
        }
        /* Same mirroring down here */
        if self.get() > 0x3FFF {
            self.set(self.get() & 0x3FFF);
        }
    }
}
