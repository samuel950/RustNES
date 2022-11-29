use crate::ppu_utils::AddressRegister::AddressRegister;
use crate::ppu_utils::ControllerRegister::ControllerRegister;
use crate::ppu_utils::MaskRegister::MaskRegister;
use crate::ppu_utils::ScrollRegister::ScrollRegister;
use crate::ppu_utils::StatusRegister::StatusFlag;
use crate::ppu_utils::StatusRegister::StatusRegister;
use crate::rom::Mirroring;
pub struct PPU {
    pub chr_rom: Vec<u8>,
    pub palette: [u8; 32],
    pub vram: [u8; 2048],
    pub mirroring: Mirroring,
    pub oam_address: u8,
    pub oam_data: [u8; 256],
    pub controller: ControllerRegister,
    pub scroll: ScrollRegister,
    pub address: AddressRegister,
    pub mask: MaskRegister,
    pub status: StatusRegister,
    pub data_buffer: u8,
    pub scanline: u16,
    pub cycles: usize,
    pub nmi_interrupt: Option<u8>,
}
impl PPU {
    pub fn new(chr_rom: Vec<u8>, mirroring: Mirroring) -> Self {
        PPU {
            chr_rom: chr_rom,
            palette: [0; 32],
            vram: [0; 2048],
            mirroring: mirroring,
            oam_address: 0,
            oam_data: [0; 64 * 4],
            controller: ControllerRegister::new(),
            scroll: ScrollRegister::new(),
            address: AddressRegister::new(),
            mask: MaskRegister::new(),
            status: StatusRegister::new(),
            data_buffer: 0,
            scanline: 0,
            cycles: 0,
            nmi_interrupt: None,
        }
    }
    pub fn tick(&mut self, cycles: u8) -> bool {
        /*println!(
            "ppu cycle: {}. ppu scanline: {}\n",
            self.cycles, self.scanline
        );*/
        self.cycles += cycles as usize;
        if self.cycles >= 341 {
            self.cycles = self.cycles - 341;
            self.scanline += 1;
            if self.scanline == 241 {
                self.status.enable_flag(&StatusFlag::VBlank);
                self.status.disable_flag(&StatusFlag::SpriteZero);
                if self.controller.generate_nmi() {
                    self.nmi_interrupt = Some(1);
                }
            }
            if self.scanline >= 262 {
                self.scanline = 0;
                self.nmi_interrupt = None;
                self.status.disable_flag(&StatusFlag::SpriteZero);
                self.status.disable_flag(&StatusFlag::VBlank);
                return true;
            }
        }

        return false;
    }
    pub fn poll_nmi(&mut self) -> Option<u8> {
        self.nmi_interrupt.take()
    }
    pub fn write_mask(&mut self, data: u8) {
        self.mask.update(data);
    }
    pub fn write_ppu_address(&mut self, data: u8) {
        self.address.update(data);
    }
    pub fn write_scroll(&mut self, data: u8) {
        self.scroll.write(data);
    }
    pub fn write_controller(&mut self, data: u8) {
        let nmi_copy = self.controller.generate_nmi();
        self.controller.update(data);
        if !nmi_copy
            && self.controller.generate_nmi()
            && self.status.get_register_status(&StatusFlag::VBlank)
        {
            self.nmi_interrupt = Some(1);
        }
    }
    pub fn read_status(&mut self) -> u8 {
        let data = self.status.snapshot();
        self.status.disable_flag(&StatusFlag::VBlank);
        self.address.reset_latch();
        self.scroll.reset_latch();
        data
    }
    fn increment_vram_address(&mut self) {
        self.address.increment(self.controller.vram_address_inc());
    }
    pub fn write_oam_address(&mut self, data: u8) {
        self.oam_address = data;
    }
    pub fn write_oam_data(&mut self, data: u8) {
        self.oam_data[self.oam_address as usize] = data;
        self.oam_address = self.oam_address.wrapping_add(1);
    }
    pub fn read_oam_data(&self) -> u8 {
        self.oam_data[self.oam_address as usize]
    }
    pub fn read_data(&mut self) -> u8 {
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
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                //various mirrored locations
                let mirror = addr - 0x10;
                self.palette[(mirror - 0x3f00) as usize]
            }
            0x3f00..=0x3fff => self.palette[(addr - 0x3f00) as usize],
            _ => panic!("Unexpected read of mirrored space at: {}", addr),
        }
    }
    pub fn write_oam_dma(&mut self, data: &[u8; 256]) {
        for x in data.iter() {
            self.oam_data[self.oam_address as usize] = *x;
            self.oam_address = self.oam_address.wrapping_add(1);
        }
    }
    pub fn write_data(&mut self, data: u8) {
        let addr = self.address.get();
        match addr {
            0..=0x1fff => {
                panic!("Attempt to write to chr rom space at: {}", addr);
            }
            0x2000..=0x2fff => {
                //mirrored location
                self.vram[self.mirror_address(addr) as usize] = data;
            }
            0x3000..=0x3eff => panic!(
                "Address space 0x3000..0x3eff is not used by PPU, requested: {}",
                addr
            ),
            0x3f10 | 0x3f14 | 0x3f18 | 0x3f1c => {
                //various mirrored locations
                let mirror = addr - 0x10;
                self.palette[(mirror - 0x3f00) as usize] = data;
            }
            0x3f00..=0x3fff => self.palette[(addr - 0x3f00) as usize] = data,
            _ => panic!("Unexpected write to mirrored space at: {}", addr),
        }
        self.increment_vram_address();
    }
    // Horizontal:
    //   [ A ] [ a ]
    //   [ B ] [ b ]

    // Vertical:
    //   [ A ] [ B ]
    //   [ a ] [ b ]
    pub fn mirror_address(&self, addr: u16) -> u16 {
        let mirrored_vram = addr & 0b10111111111111; //shift 0x3000-3eff to 0x2000-0x2eff
        let vram_index = mirrored_vram - 0x2000;
        let name_table = vram_index / 0x400;
        match (&self.mirroring, name_table) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 1) => vram_index - 0x400,
            (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x400,
            _ => vram_index,
        }
    }
}
