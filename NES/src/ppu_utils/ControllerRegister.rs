/*VPHB SINN
|||| ||||
|||| ||++- Base nametable address
|||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
|||| |+--- VRAM address increment per CPU read/write of PPUDATA
|||| |     (0: add 1, going across; 1: add 32, going down)
|||| +---- Sprite pattern table address for 8x8 sprites
||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
|||+------ Background pattern table address (0: $0000; 1: $1000)
||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
|+-------- PPU master/slave select
|          (0: read backdrop from EXT pins; 1: output color on EXT pins)
+--------- Generate an NMI at the start of the
           vertical blanking interval (0: off; 1: on)
*/
pub struct ControllerRegister {
    pub cregister: u8,
}
pub enum ControllerFlag {
    Nametable1,
    Nametable2,
    VramAddress,
    SpriteAddress,
    BackgroundAddress,
    SpriteSize,
    MasterSlaveSelect,
    GenerateNmi,
}
impl ControllerRegister {
    pub fn new() -> Self {
        ControllerRegister {
            cregister: 0b0000_0000,
        }
    }
    pub fn nametable_address(&self) -> u16 {
        match self.cregister & 0b11 {
            0 => 0x2000,
            1 => 0x2400,
            2 => 0x2800,
            3 => 0x2c00,
            _ => panic!("Invalid nametable address!"),
        }
    }
    pub fn vram_address_inc(&self) -> u8 {
        if !self.get_register_status(&ControllerFlag::VramAddress) {
            1
        } else {
            32
        }
    }
    pub fn sprite_address(&self) -> u16 {
        if !self.get_register_status(&ControllerFlag::SpriteAddress) {
            0
        } else {
            0x1000
        }
    }
    pub fn background_address(&self) -> u16 {
        if !self.get_register_status(&ControllerFlag::BackgroundAddress) {
            0
        } else {
            0x1000
        }
    }
    pub fn sprite_size(&self) -> u8 {
        if !self.get_register_status(&ControllerFlag::SpriteSize) {
            8
        } else {
            16
        }
    }
    pub fn ms_select(&self) -> u8 {
        if !self.get_register_status(&ControllerFlag::MasterSlaveSelect) {
            0
        } else {
            1
        }
    }
    pub fn generate_nmi(&self) -> bool {
        self.get_register_status(&ControllerFlag::GenerateNmi)
    }
    pub fn update(&mut self, data: u8) {
        self.cregister = data;
    }
    pub fn get_register_status(&self, cflag: &ControllerFlag) -> bool {
        match cflag {
            ControllerFlag::Nametable1 => self.cregister & 0b0000_0001 != 0,
            ControllerFlag::Nametable2 => self.cregister & 0b0000_0010 != 0,
            ControllerFlag::VramAddress => self.cregister & 0b0000_0100 != 0,
            ControllerFlag::SpriteAddress => self.cregister & 0b0000_1000 != 0,
            ControllerFlag::BackgroundAddress => self.cregister & 0b0001_0000 != 0,
            ControllerFlag::SpriteSize => self.cregister & 0b0010_0000 != 0,
            ControllerFlag::MasterSlaveSelect => self.cregister & 0b0100_0000 != 0,
            ControllerFlag::GenerateNmi => self.cregister & 0b1000_0000 != 0,
        }
    }
}
