pub struct Pulse {
    pub bits0: u8,
    pub bits1: u8,
    pub bits2: u8,
    pub bits3: u8,
}
impl Pulse {
    pub fn new() -> Self {
        Pulse {
            bits0: 0b0000_0000,
            bits1: 0b0000_0000,
            bits2: 0b0000_0000,
            bits3: 0b0000_0000,
        }
    }
}
