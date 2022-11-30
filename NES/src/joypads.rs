pub enum Button {
    Right,
    Left,
    Down,
    Up,
    Start,
    Select,
    B,
    A,
    None,
}
pub struct Joypad {
    pub button_status: u8,
    pub strobe: bool,
    pub bidx: u8,
}
impl Joypad {
    pub fn new() -> Self {
        Joypad {
            button_status: 0,
            strobe: false,
            bidx: 0,
        }
    }
    pub fn write(&mut self, data: u8) {
        self.strobe = data & 1 == 1;
        if self.strobe {
            self.bidx = 0;
        }
    }
    pub fn read(&mut self) -> u8 {
        if self.bidx > 7 {
            return 1;
        }
        let resp = (self.button_status & (1 << self.bidx)) >> self.bidx;
        if !self.strobe && self.bidx <= 7 {
            self.bidx += 1;
        }
        resp
    }
    pub fn set_button(&mut self, button: &Button, pressed: bool) {
        if pressed {
            self.button_status = self.get_button(button);
        } else {
            self.button_status = 0;
        }
    }
    pub fn get_button(&self, button: &Button) -> u8 {
        match button {
            Button::Right => 0b1000_0000,
            Button::Left => 0b0100_0000,
            Button::Down => 0b0010_0000,
            Button::Up => 0b0001_0000,
            Button::Start => 0b0000_1000,
            Button::Select => 0b0000_0100,
            Button::B => 0b0000_0010,
            Button::A => 0b0000_0001,
            Button::None => 0b0000_0000,
        }
    }
}
