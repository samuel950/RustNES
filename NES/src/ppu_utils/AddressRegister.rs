pub struct AddressRegister {
    value: (u8, u8),
    hi_ptr: bool,
}
impl AddressRegister {
    pub fn new() -> Self {
        AddressRegister {
            value: (0, 0),
            hi_ptr: true,
        }
    }
    fn set(&mut self, data: u16) {
        self.value.0 = (data >> 8) as u8;
        self.value.1 = (data & 0xff) as u8;
    }
    pub fn update(&mut self, data: u8) {
        if self.hi_ptr {
            self.value.0 = data;
        } else {
            self.value.1 = data;
        }
        let t = self.get();
        if t > 0x3fff {
            self.set(t & 0b11111111111111);
        }
        self.hi_ptr = !self.hi_ptr;
    }
    pub fn increment(&mut self, inc: u8) {
        let lo = self.value.1;
        self.value.1 = self.value.1.wrapping_add(inc);
        if lo > self.value.1 {
            self.value.0 = self.value.0.wrapping_add(1);
        }
        let t = self.get();
        if t > 0x3fff {
            self.set(t & 0b11111111111111);
        }
    }
    pub fn reset_latch(&mut self) {
        self.hi_ptr = true;
    }
    pub fn get(&self) -> u16 {
        ((self.value.0 as u16) << 8) | self.value.1 as u16
    }
}
