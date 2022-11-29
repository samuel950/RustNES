/*
VSO. ....
|||| ||||
|||+-++++- PPU open bus. Returns stale PPU bus contents.
||+------- Sprite overflow. The intent was for this flag to be set
||         whenever more than eight sprites appear on a scanline, but a
||         hardware bug causes the actual behavior to be more complicated
||         and generate false positives as well as false negatives; see
||         PPU sprite evaluation. This flag is set during sprite
||         evaluation and cleared at dot 1 (the second dot) of the
||         pre-render line.
|+-------- Sprite 0 Hit.  Set when a nonzero pixel of sprite 0 overlaps
|          a nonzero background pixel; cleared at dot 1 of the pre-render
|          line.  Used for raster timing.
+--------- Vertical blank has started (0: not in vblank; 1: in vblank).
           Set at dot 1 of line 241 (the line *after* the post-render
           line); cleared after reading $2002 and at dot 1 of the
           pre-render line.
*/
pub struct StatusRegister {
    pub sregister: u8,
}
pub enum StatusFlag {
    VBlank,
    SpriteZero,
    SpriteOverflow,
}
impl StatusRegister {
    pub fn new() -> Self {
        StatusRegister {
            sregister: 0b0000_0000,
        }
    }
    pub fn enable_flag(&mut self, sflag: &StatusFlag) {
        match sflag {
            StatusFlag::SpriteOverflow => self.sregister = self.sregister | 0b0010_0000,
            StatusFlag::SpriteZero => self.sregister = self.sregister | 0b0100_0000,
            StatusFlag::VBlank => self.sregister = self.sregister | 0b1000_0000,
        }
    }
    pub fn disable_flag(&mut self, sflag: &StatusFlag) {
        match sflag {
            StatusFlag::SpriteOverflow => self.sregister = self.sregister & 0b1101_1111,
            StatusFlag::SpriteZero => self.sregister = self.sregister & 0b1011_1111,
            StatusFlag::VBlank => self.sregister = self.sregister & 0b0111_1111,
        }
    }
    pub fn get_register_status(&self, sflag: &StatusFlag) -> bool {
        match sflag {
            StatusFlag::SpriteOverflow => self.sregister & 0b0010_0000 != 0,
            StatusFlag::SpriteZero => self.sregister & 0b0100_0000 != 0,
            StatusFlag::VBlank => self.sregister & 0b1000_0000 != 0,
        }
    }
    pub fn snapshot(&self) -> u8 {
        self.sregister
    }
}
