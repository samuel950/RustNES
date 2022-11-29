pub struct Frame {
    pub data: Vec<u8>,
}
impl Frame {
    const W: usize = 256;
    const H: usize = 240;

    pub fn new() -> Self {
        Frame {
            data: vec![0; (Frame::W) * (Frame::H) * 3],
        }
    }
    //abstractionlayer to avoid direction ineraction with SDL
    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        let base = y * 3 * Frame::W + x * 3;
        if base + 2 < self.data.len() {
            self.data[base] = rgb.0;
            self.data[base + 1] = rgb.1;
            self.data[base + 2] = rgb.2;
        }
    }
}
