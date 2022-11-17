use crate::rom::Rom;
pub struct Bus {
    cpu_vram: [u8; 2048],
    rom: Rom,
}
pub trait Memory {
    fn mem_read(&self, addr: u16) -> u8;
    fn mem_write(&mut self, addr: u16, data: u8);
    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        //let hi = self.mem_read(pos + 1) as u16; //visit the next cell to grab the last 8 bits of data.
        let hi = self.mem_read(pos.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }
    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8; //basically shifts hi 8 bits to the right to allow truncating
        let lo = (data & 0xff) as u8; //zero out 8 hi bits to allow truncating
        self.mem_write(pos, lo);
        //self.mem_write(pos + 1, hi);
        self.mem_write(pos.wrapping_add(1), hi);
    }
}
const RAM: u16 = 0x0000;
const RAM_MIRROR: u16 = 0x1FFF;
const PPU_REGISTERS: u16 = 0x2000;
const PPU_REGISTERS_MIRROR: u16 = 0x3FFF;

impl Bus {
    pub fn new(rom: Rom) -> Self {
        Bus {
            cpu_vram: [0; 2048],
            rom: rom,
        }
    }
    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.rom.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            //mirroring
            addr = addr % 0x4000;
        }
        self.rom.prg_rom[addr as usize]
    }
}
impl Memory for Bus {
    fn mem_read(&self, addr: u16) -> u8 {
        match addr {
            RAM..=RAM_MIRROR => {
                let mirror_addr = addr & 0b00000111_11111111;
                self.cpu_vram[mirror_addr as usize]
            }
            PPU_REGISTERS..=PPU_REGISTERS_MIRROR => {
                let _ppu_mirror_addr = addr & 0b00100000_00000111;
                todo!("PPU not yet implemented")
            }
            0x8000..=0xFFFF => self.read_prg_rom(addr),
            _ => {
                println!("Cannot read memory at {}!", addr);
                0
            }
        }
    }
    fn mem_write(&mut self, addr: u16, data: u8) {
        match addr {
            RAM..=RAM_MIRROR => {
                let mirror_addr = addr & 0b00000111_11111111;
                self.cpu_vram[mirror_addr as usize] = data;
            }
            PPU_REGISTERS..=PPU_REGISTERS_MIRROR => {
                let _ppu_mirror_addr = addr & 0b00100000_00000111;
                todo!("PPU not yet implemented")
            }
            0x8000..=0xFFFF => {
                panic!("Attempt to write to cartridge ROM space!")
            }
            _ => {
                println!("Cannot write {} to {}!", data, addr);
            }
        }
    }
}
