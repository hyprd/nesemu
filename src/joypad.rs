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
    button_status: u8,
    button_index: JoypadButton,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            strobe_status : false,
            button_status: 0,
            button_index: JoypadButton::from_bits_truncate(0b00000000),
        }
    }
}
