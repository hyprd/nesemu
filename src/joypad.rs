use bitflags::Flags;

bitflags! {
    pub struct JoypadButton: u8 {
        const A = 0b00;
        const B = 0b01;
        const SELECT = 0b10;
        const START = 0b11;
        const UP = 0b100;
        const DOWN = 0b101;
        const LEFT = 0b110;
        const RIGHT = 0b111;
    }
}

pub struct Joypad {
    strobe_status: bool,
    button_status: JoypadButton,
    button_index: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            strobe_status : false,
            button_index: 0,
            button_status: JoypadButton::from_bits_truncate(0b00000000),
        }
    }

    pub fn read(&mut self) -> u8 {
        if self.button_index > 7 {
            return 1;
        }
        let result = (self.button_status.bits() & (1 << self.button_index)) >> self.button_index;
        if !self.strobe_status && self.button_index <= 7 {
            self.button_index += 1;
        }
        result
    }

    pub fn write(&mut self, value: u8) {
        // While strobe is high, continuously return state of button A (0b0)
        if value & 0x01 == 1 {
            self.strobe_status = true;
            self.button_index = 0b0;
        }
    }
}
