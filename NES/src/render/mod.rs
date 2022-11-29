pub mod frame;
pub mod palette;
use crate::ppu::PPU;
use frame::Frame;

pub fn render(ppu: &PPU, frame: &mut Frame) {
    let bank = ppu.controller.background_address();
    for i in 0..0x03c0 {
        //0x03c0 using first nametable only
        let tile = ppu.vram[i] as u16;
        let xtile = i % 32;
        let ytile = i / 32;
        let tile = &ppu.chr_rom[(bank + tile * 16) as usize..=(bank + tile * 16 * 15) as usize];
        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            for x in (0..=7).rev() {
                let value = (1 & upper) << 1 | (1 & lower);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => palette::SYSTEM_PALLETE[0x01],
                    1 => palette::SYSTEM_PALLETE[0x23],
                    2 => palette::SYSTEM_PALLETE[0x27],
                    3 => palette::SYSTEM_PALLETE[0x30],
                    _ => panic!("Invalid color!"),
                };
                frame.set_pixel(xtile * 8 + x, ytile * 8 + y, rgb)
            }
        }
    }
}
