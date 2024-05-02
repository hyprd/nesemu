use bitflags::Flags;

bitflags! {
    #[derive(Copy, Clone)]
    pub struct JoypadButton: u8 {
        const RIGHT = 0b10000000;
        const LEFT = 0b01000000;
        const DOWN = 0b00100000;
        const UP = 0b00010000;
        const START = 0b00001000;
        const SELECT = 0b00000100;
        const B = 0b00000010;
        const A = 0b00000001;
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
            strobe_status: false,
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
        self.strobe_status = value & 0x01 == 1;
        if self.strobe_status {
            self.button_index = 0;
        }
    }

    pub fn set_pressed(&mut self, button: JoypadButton, pressed: bool) {
        self.button_status.set(button, pressed);
        println!("Button {}", button.bits());
    }
}
