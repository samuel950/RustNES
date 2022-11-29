pub struct ScrollRegister {
    pub xscroll: u8,
    pub yscroll: u8,
    pub latch: bool,
}
impl ScrollRegister {
    pub fn new() -> Self {
        ScrollRegister {
            xscroll: 0,
            yscroll: 0,
            latch: false,
        }
    }
    pub fn write(&mut self, data: u8) {
        if !self.latch {
            self.xscroll = data;
        } else {
            self.yscroll = data;
        }
        self.latch = !self.latch;
    }
    pub fn reset_latch(&mut self) {
        self.latch = false;
    }
}
