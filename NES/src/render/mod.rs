pub mod frame;
pub mod palette;
use crate::ppu::PPU;
use frame::Frame;

fn sprite_palette(ppu: &PPU, palidx: u8) -> [u8; 4] {
    let i = 0x11 + (palidx * 4) as usize;
    [0, ppu.palette[i], ppu.palette[i + 1], ppu.palette[i + 2]]
}
fn background_palette(ppu: &PPU, x: usize, y: usize) -> [u8; 4] {
    let atidx = y / 4 * 8 + x / 4;
    let atbyte = ppu.vram[0x3c0 + atidx]; //0x3c0 only sing first nametable need to adjust later
    let palidx = match (x % 4 / 2, y % 4 / 2) {
        (0, 0) => atbyte & 0b11,
        (1, 0) => (atbyte >> 2) & 0b11,
        (0, 1) => (atbyte >> 4) & 0b11,
        (1, 1) => (atbyte >> 6) & 0b11,
        (j, k) => panic!("background_palette invalid state: {}, {}", j, k),
    };
    let i: usize = 1 + (palidx as usize) * 4;
    [0, ppu.palette[i], ppu.palette[i + 1], ppu.palette[i + 2]]
}
pub fn render(ppu: &PPU, frame: &mut Frame) {
    let bank = ppu.controller.background_address();
    for i in 0..0x03c0 {
        //0x03c0 using first nametable only
        let tile = ppu.vram[i] as u16;
        let xtile = i % 32;
        let ytile = i / 32;
        let tile = &ppu.chr_rom[(bank + tile * 16) as usize..=(bank + tile * 16 + 15) as usize];
        let bp = background_palette(ppu, xtile, ytile);
        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => palette::SYSTEM_PALLETE[ppu.palette[0] as usize],
                    1 => palette::SYSTEM_PALLETE[bp[1] as usize],
                    2 => palette::SYSTEM_PALLETE[bp[2] as usize],
                    3 => palette::SYSTEM_PALLETE[bp[3] as usize],
                    _ => panic!("Invalid color!"),
                };
                frame.set_pixel(xtile * 8 + x, ytile * 8 + y, rgb)
            }
        }
    }
    for j in (0..ppu.oam_data.len()).step_by(4).rev() {
        let tidx = ppu.oam_data[j + 1] as u16;
        let xtile = ppu.oam_data[j + 3] as usize;
        let ytile = ppu.oam_data[j] as usize;
        let vertical_rotate = if ppu.oam_data[j + 2] >> 7 & 1 == 1 {
            true
        } else {
            false
        };
        let horizontal_rotate = if ppu.oam_data[j + 2] >> 6 & 1 == 1 {
            true
        } else {
            false
        };
        let palidx = ppu.oam_data[j + 2] & 0b11;
        let sp = sprite_palette(ppu, palidx);
        let bank = ppu.controller.sprite_address();
        let tile = &ppu.chr_rom[(bank + tidx * 16) as usize..=(bank + tidx * 16 + 15) as usize];

        for y in 0..=7 {
            let mut upper = tile[y];
            let mut lower = tile[y + 8];
            'inner: for x in (0..=7).rev() {
                let value = (1 & lower) << 1 | (1 & upper);
                upper = upper >> 1;
                lower = lower >> 1;
                let rgb = match value {
                    0 => continue 'inner, //skip this pixel
                    1 => palette::SYSTEM_PALLETE[sp[1] as usize],
                    2 => palette::SYSTEM_PALLETE[sp[2] as usize],
                    3 => palette::SYSTEM_PALLETE[sp[3] as usize],
                    _ => panic!("Invalid color!"),
                };
                match (horizontal_rotate, vertical_rotate) {
                    (false, false) => frame.set_pixel(xtile + x, ytile + y, rgb),
                    (true, false) => frame.set_pixel(xtile + 7 - x, ytile + y, rgb),
                    (false, true) => frame.set_pixel(xtile + x, ytile + 7 - y, rgb),
                    (true, true) => frame.set_pixel(xtile + 7 - x, ytile + 7 - y, rgb),
                }
            }
        }
    }
}
