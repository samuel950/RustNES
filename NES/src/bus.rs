use crate::joypads::Joypad;
use crate::ppu::PPU;
use crate::rom::Rom;
pub struct Bus<'call> {
    cpu_vram: [u8; 2048],
    prg_rom: Vec<u8>,
    pub ppu: PPU,
    pub cycles: usize,
    game_callback: Box<dyn FnMut(&PPU, &mut Joypad) + 'call>,
    joypad1: Joypad,
}
pub trait Memory {
    fn mem_read(&mut self, addr: u16) -> u8;
    fn mem_write(&mut self, addr: u16, data: u8);
    fn mem_read_u16(&mut self, pos: u16) -> u16 {
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

impl<'a> Bus<'a> {
    pub fn new<'call, F>(rom: Rom, game_callback: F) -> Bus<'call>
    where
        F: FnMut(&PPU, &mut Joypad) + 'call,
    {
        let ppu = PPU::new(rom.chr_rom, rom.screen_mirroring);
        Bus {
            cpu_vram: [0; 2048],
            prg_rom: rom.prg_rom,
            ppu: ppu,
            cycles: 0,
            game_callback: Box::from(game_callback),
            joypad1: Joypad::new(),
        }
    }
    fn read_prg_rom(&self, mut addr: u16) -> u8 {
        addr -= 0x8000;
        if self.prg_rom.len() == 0x4000 && addr >= 0x4000 {
            //mirroring
            addr = addr % 0x4000;
        }
        self.prg_rom[addr as usize]
    }
    pub fn tick(&mut self, cycles: u8) {
        //println!("bus cycles: {}", self.cycles);
        self.cycles += cycles as usize;
        let before_nmi = self.ppu.nmi_interrupt.is_some();
        self.ppu.tick(cycles * 3);
        let after_nmi = self.ppu.nmi_interrupt.is_some();
        if !before_nmi && after_nmi {
            (self.game_callback)(&self.ppu, &mut self.joypad1);
        }
    }
    pub fn poll_nmi(&mut self) -> Option<u8> {
        self.ppu.poll_nmi()
    }
}
impl Memory for Bus<'_> {
    fn mem_read(&mut self, addr: u16) -> u8 {
        match addr {
            RAM..=RAM_MIRROR => {
                let mirror_addr = addr & 0b00000111_11111111;
                self.cpu_vram[mirror_addr as usize]
            }
            0x2000 | 0x2001 | 0x2003 | 0x2005 | 0x2006 | 0x4014 => {
                println!("Attempt to read from write-only PPU address {:x}!", addr);
                0
            }
            0x2002 => self.ppu.read_status(),
            0x2004 => self.ppu.read_oam_data(),
            0x2007 => self.ppu.read_data(),
            0x2008..=PPU_REGISTERS_MIRROR => {
                let ppu_mirror_addr = addr & 0b00100000_00000111;
                self.mem_read(ppu_mirror_addr)
            }
            0x4000..=0x4015 => {
                //apu not yet implemented
                0
            }
            0x4016 => {
                //joypad 1
                self.joypad1.read()
            }
            0x4017 => {
                //joypad 2
                0
            }
            0x8000..=0xFFFF => self.read_prg_rom(addr),
            _ => {
                println!("Cannot read memory at {:x}!", addr);
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
            0x2000 => {
                self.ppu.write_controller(data);
            }
            0x2001 => {
                self.ppu.write_mask(data);
            }
            0x2002 => {
                panic!("Attempt to write to PPU Status register!");
            }
            0x2003 => {
                self.ppu.write_oam_address(data);
            }
            0x2004 => {
                self.ppu.write_oam_data(data);
            }
            0x2005 => {
                self.ppu.write_scroll(data);
            }
            0x2006 => {
                self.ppu.write_ppu_address(data);
            }
            0x2007 => {
                self.ppu.write_data(data);
            }
            0x2008..=PPU_REGISTERS_MIRROR => {
                let ppu_mirror_addr = addr & 0b00100000_00000111;
                self.mem_write(ppu_mirror_addr, data);
            }
            0x4000..=0x4013 | 0x4015 => {
                //apu not yet implemented
            }
            0x4016 => {
                //joypad 1
                self.joypad1.write(data);
            }
            0x4017 => {
                //joypad 2
            }
            0x4014 => {
                //oam dma
                let mut buffer: [u8; 256] = [0; 256];
                let hi = (data as u16) << 8;
                for i in 0..256u16 {
                    buffer[i as usize] = self.mem_read(hi + i);
                }
                self.ppu.write_oam_dma(&buffer);
                //must implement correct cycle logic for oam dma write.
            }
            0x8000..=0xFFFF => {
                panic!("Attempt to write to cartridge ROM space!")
            }
            _ => {
                println!("Cannot write {} to {:x}!", data, addr);
            }
        }
    }
}
