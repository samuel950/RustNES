use rom::Mirroring;
struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette: [u8; 32],
    pub vram: [u8; 2048],
    pub oam_data: [u8; 256],
    pub mirroring: Mirroring,
    pub address: AddressRegister,
    pub controller: ControllerRegister,
    pub data_buffer: u8,
}
struct AddressRegister {
    value: (u8, u8),
    hi_ptr: bool,
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
pub struct ControllerRegister {
    pub cregister: u8,
}
impl PPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPU {
            chr_rom: chr_rom,
            mirroring: mirroring,
            vram: [0; 2048],
            oam_data: [0; 64 * 4],
            palette_table: [0; 32],
        }
    }
    fn write_ppu_address(&mut self, value: u8) {
        self.address.update(value);
    }
    fn write_controller(&mut self, value: u8) {
        self.ctrl.update(value);
    }
    fn increment_vram_address(&mut self) {
        self.address.increment(self.controller.vram_address_inc());
    }
    fn read_data(&mut self) -> u8 {
        let addr = self.address.get();
        self.increment_vram_address();
        match addr {
            0..=0x1fff => {
                let result = self.data_buffer;
                self.data_buffer = self.chr_rom[addr as usize];
                result
            }
            0x2000..=0x2fff => {
                //mirrored location
                let result = self.data_buffer;
                self.data_buffer = self.vram[self.mirror_address(addr) as usize];
                result
            }
            0x3000..=0x3eff => panic!(
                "Address space 0x3000..0x3eff is not used by PPU, requested: {}",
                addr
            ),
            0x3f00..=0x3fff => self.palette_table[(addr - 0x3f00) as usize],
            _ => panic!("Unexpected access to mirrored space at: {}", addr),
        }
    }
    // Horizontal:
    //   [ A ] [ a ]
    //   [ B ] [ b ]

    // Vertical:
    //   [ A ] [ B ]
    //   [ a ] [ b ]
    pub fn mirror_address(&self, addr: u16) -> u16 {}
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
impl ControllerRegister {
    pub fn new() -> self {
        self.cregister = 0b0000_0000;
    }
    pub fn vram_address_inc(&self) -> u8 {
        if !self.get_register_status(&ControllerFlag::VramAddress) {
            1
        } else {
            32
        }
    }
    pub fn update(&mut self, data: u8) {
        self.cregister = data;
    }
    pub fn get_register_status(&self, cflag: &ControllerFlag) -> bool {
        match cflag {
            ControllerFlag::Nametable1 => self.status & 0b0000_0001 != 0,
            ControllerFlag::Nametable2 => self.status & 0b0000_0010 != 0,
            ControllerFlag::VramAddress => self.status & 0b0000_0100 != 0,
            ControllerFlag::SpriteAddress => self.status & 0b0000_1000 != 0,
            ControllerFlag::BackgroundAddress => self.status & 0b0001_0000 != 0,
            ControllerFlag::SpriteSize => self.status & 0b0010_0000 != 0,
            ControllerFlag::MasterSlaveSelect => self.status & 0b0100_0000 != 0,
            ControllerFlag::GenerateNmi => self.status & 0b1000_0000 != 0,
        }
    }
}
