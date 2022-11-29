/*
BGRs bMmG
|||| ||||
|||| |||+- Greyscale (0: normal color, 1: produce a greyscale display)
|||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
|||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
|||| +---- 1: Show background
|||+------ 1: Show sprites
||+------- Emphasize red (green on PAL/Dendy)
|+-------- Emphasize green (red on PAL/Dendy)
+--------- Emphasize blue
 */
pub struct MaskRegister {
    pub mregister: u8,
}
pub enum MaskFlag {
    Blue,
    Green,
    Red,
    Sprite,
    Background,
    LeftSprite,
    LeftBackground,
    Greyscale,
}
pub enum Color {
    Blue,
    Green,
    Red,
}
impl MaskRegister {
    pub fn new() -> Self {
        MaskRegister {
            mregister: 0b0000_0000,
        }
    }
    pub fn get_register_status(&self, mflag: &MaskFlag) -> bool {
        match mflag {
            MaskFlag::Greyscale => self.mregister & 0b0000_0001 != 0,
            MaskFlag::LeftBackground => self.mregister & 0b0000_0010 != 0,
            MaskFlag::LeftSprite => self.mregister & 0b0000_0100 != 0,
            MaskFlag::Background => self.mregister & 0b0000_1000 != 0,
            MaskFlag::Sprite => self.mregister & 0b0001_0000 != 0,
            MaskFlag::Red => self.mregister & 0b0010_0000 != 0,
            MaskFlag::Green => self.mregister & 0b0100_0000 != 0,
            MaskFlag::Blue => self.mregister & 0b1000_0000 != 0,
        }
    }
    pub fn emphasise(&self) -> Vec<Color> {
        let mut result = Vec::<Color>::new();
        if self.get_register_status(&MaskFlag::Blue) {
            result.push(Color::Blue);
        }
        if self.get_register_status(&MaskFlag::Green) {
            result.push(Color::Green);
        }
        if self.get_register_status(&MaskFlag::Red) {
            result.push(Color::Red);
        }
        result
    }
    pub fn update(&mut self, data: u8) {
        self.mregister = data;
    }
}
