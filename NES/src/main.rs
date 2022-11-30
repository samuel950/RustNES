pub mod TileViewer;
pub mod bus;
pub mod cpu;
pub mod joypads;
pub mod opcodes;
pub mod ppu;
pub mod ppu_utils;
pub mod render;
pub mod rom;
use bus::Bus;
use bus::Memory;
use cpu::AddressingMode;
use cpu::CPU;
use joypads::Button;
use ppu::PPU;
use render::frame::Frame;
use rom::Rom;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::EventPump;
use std::collections::HashMap;
use std::env;

fn input_handler(cpu: &mut CPU, event_pump: &mut EventPump) {
    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. }
            | Event::KeyDown {
                keycode: Some(Keycode::Escape),
                ..
            } => std::process::exit(0),
            Event::KeyDown {
                keycode: Some(Keycode::W),
                ..
            } => {
                cpu.mem_write(0xff, 0x77);
            }
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } => {
                cpu.mem_write(0xff, 0x61);
            }
            Event::KeyDown {
                keycode: Some(Keycode::S),
                ..
            } => {
                cpu.mem_write(0xff, 0x73);
            }
            Event::KeyDown {
                keycode: Some(Keycode::D),
                ..
            } => {
                cpu.mem_write(0xff, 0x64);
            }
            _ => { /*do nothing*/ }
        }
    }
}
fn color(byte: u8) -> Color {
    match byte {
        0 => sdl2::pixels::Color::BLACK,
        1 => sdl2::pixels::Color::WHITE,
        2 | 9 => sdl2::pixels::Color::GREY,
        3 | 10 => sdl2::pixels::Color::RED,
        4 | 11 => sdl2::pixels::Color::GREEN,
        5 | 12 => sdl2::pixels::Color::BLUE,
        6 | 13 => sdl2::pixels::Color::MAGENTA,
        7 | 14 => sdl2::pixels::Color::YELLOW,
        _ => sdl2::pixels::Color::CYAN,
    }
}
fn read_screen_state(cpu: &mut CPU, frame: &mut [u8; 32 * 3 * 32]) -> bool {
    let mut frame_idx = 0;
    let mut update = false;
    for i in 0x0200..0x600 {
        let color_idx = cpu.mem_read(i as u16);
        let (b1, b2, b3) = color(color_idx).rgb();
        if frame[frame_idx] != b1 || frame[frame_idx + 1] != b2 || frame[frame_idx + 2] != b3 {
            frame[frame_idx] = b1;
            frame[frame_idx + 1] = b2;
            frame[frame_idx + 2] = b3;
            update = true;
        }
        frame_idx += 3;
    }
    update
}
fn trace(cpu: &mut CPU) -> String {
    let ref opscodes: HashMap<u8, &'static opcodes::Opcode> = *opcodes::OPCODES_MAP;
    let program_counter = cpu.program_counter;
    let register_a = cpu.register_a;
    let register_x = cpu.register_x;
    let register_y = cpu.register_y;
    let p = cpu.status;
    let sp = cpu.stack_ptr;
    let opscode = cpu.mem_read(program_counter);
    let opscode_data = opscodes.get(&opscode).unwrap();
    let registers = format!(
        "A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x}",
        register_a, register_x, register_y, p, sp
    );
    let ppuSL = cpu.bus.ppu.scanline;
    if ppuSL == 242 {
        std::process::exit(0);
    }
    let ppucyc = cpu.bus.ppu.cycles;
    let buscyc = cpu.bus.cycles;
    let ppuinfo = format!("PPU:{:3},{:3} CYC:{}", ppuSL, ppucyc, buscyc);
    let mut hex_dump = vec![];
    hex_dump.push(opscode);

    let (mem_addr, stored_value) = match opscode_data.mode {
        AddressingMode::Immediate | AddressingMode::NotSupported => (0, 0),
        _ => {
            let addr =
                cpu.get_operand_addressing_mode_trace(&opscode_data.mode, program_counter + 1);
            (addr, cpu.mem_read(addr))
        }
    };

    let tmp = match opscode_data.len {
        1 => match opscode_data.code {
            0x0a | 0x4a | 0x2a | 0x6a => format!("A "),
            _ => String::from(""),
        },
        2 => {
            let address: u8 = cpu.mem_read(program_counter + 1);
            // let value = cpu.mem_read(address));
            hex_dump.push(address);

            match opscode_data.mode {
                AddressingMode::Immediate => format!("#${:02x}", address),
                AddressingMode::ZeroPage => format!("${:02x} = {:02x}", mem_addr, stored_value),
                AddressingMode::ZeroPage_X => format!(
                    "${:02x},X @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::ZeroPage_Y => format!(
                    "${:02x},Y @ {:02x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::Indirect_X => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    address,
                    (address.wrapping_add(cpu.register_x)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::Indirect_Y => format!(
                    "(${:02x}),Y = {:04x} @ {:04x} = {:02x}",
                    address,
                    (mem_addr.wrapping_sub(cpu.register_y as u16)),
                    mem_addr,
                    stored_value
                ),
                AddressingMode::NotSupported => {
                    // assuming local jumps: BNE, BVS, etc....
                    let address: usize =
                        (program_counter as usize + 2).wrapping_add((address as i8) as usize);
                    format!("${:04x}", address)
                }

                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 2. code {:02x}",
                    opscode_data.mode, opscode_data.code
                ),
            }
        }
        3 => {
            let address_lo = cpu.mem_read(program_counter + 1);
            let address_hi = cpu.mem_read(program_counter + 2);
            hex_dump.push(address_lo);
            hex_dump.push(address_hi);

            let address = cpu.mem_read_u16(program_counter + 1);

            match opscode_data.mode {
                AddressingMode::NotSupported => {
                    if opscode_data.code == 0x6c {
                        //jmp indirect
                        let jmp_addr = if address & 0x00FF == 0x00FF {
                            let lo = cpu.mem_read(address);
                            let hi = cpu.mem_read(address & 0xFF00);
                            (hi as u16) << 8 | (lo as u16)
                        } else {
                            cpu.mem_read_u16(address)
                        };

                        // let jmp_addr = cpu.mem_read_u16(address);
                        format!("(${:04x}) = {:04x}", address, jmp_addr)
                    } else {
                        format!("${:04x}", address)
                    }
                }
                AddressingMode::Absolute => format!("${:04x} = {:02x}", mem_addr, stored_value),
                AddressingMode::Absolute_X => format!(
                    "${:04x},X @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                AddressingMode::Absolute_Y => format!(
                    "${:04x},Y @ {:04x} = {:02x}",
                    address, mem_addr, stored_value
                ),
                _ => panic!(
                    "unexpected addressing mode {:?} has ops-len 3. code {:02x}",
                    opscode_data.mode, opscode_data.code
                ),
            }
        }
        _ => String::from(""),
    };

    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!(
        "{:04x}  {:8} {: >4} {}",
        program_counter, hex_str, opscode_data.name, tmp
    )
    .trim()
    .to_string();
    format!("{:47} {} {}", asm_str, registers, ppuinfo).to_ascii_uppercase()
}
fn main() {
    env::set_var("RUST_BACKTRACE", "full");
    let mut key_map = HashMap::new();
    key_map.insert(Keycode::S, &joypads::Button::Down);
    key_map.insert(Keycode::W, &joypads::Button::Up);
    key_map.insert(Keycode::D, &joypads::Button::Right);
    key_map.insert(Keycode::A, &joypads::Button::Left);
    key_map.insert(Keycode::U, &joypads::Button::Select);
    key_map.insert(Keycode::I, &joypads::Button::Start);
    key_map.insert(Keycode::J, &joypads::Button::A);
    key_map.insert(Keycode::K, &joypads::Button::B);
    //init sdl2
    let sdl_context = sdl2::init().unwrap();
    let video_subsytem = sdl_context.video().unwrap();
    let window = video_subsytem
        .window("Game", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();
    //texture for render
    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();
    //load program
    let bytes: Vec<u8> = std::fs::read("../../pacman.nes").unwrap();
    let rom = Rom::new(&bytes).unwrap();
    let mut frame = Frame::new();
    //game cycle
    let bus = Bus::new(rom, move |ppu: &PPU, joypad1: &mut joypads::Joypad| {
        render::render(ppu, &mut frame);
        texture.update(None, &frame.data, 256 * 3).unwrap();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => std::process::exit(0),
                Event::KeyDown { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad1.set_button(*key, true);
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(key) = key_map.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad1.set_button(*key, false);
                    }
                }
                _ => { /* do nothing */ }
            }
        }
        ::std::thread::sleep(std::time::Duration::from_nanos(5000000));
    });
    let mut cpu = CPU::new(bus);
    cpu.reset();
    //cpu.program_counter = 0xC000; //for nesttest rom
    cpu.bus.tick(7); //for nestest rom
    cpu.run();
    /*cpu.run_with_callback(move |cpu| {
        println!("{}", trace(cpu));
        ::std::thread::sleep(std::time::Duration::from_nanos(100000000));
    });*/

    /*
    //run program
    let mut screen_state = [0 as u8; 32 * 3 * 32];
    let mut rng = rand::thread_rng();
    cpu.run_with_callback(move |cpu| {
        //println!("{}", trace(cpu));//for nesttest rom
        input_handler(cpu, &mut event_pump);
        cpu.mem_write(0xfe, rng.gen_range(1..16));
        if read_screen_state(cpu, &mut screen_state) {
            texture.update(None, &screen_state, 32 * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
            ::std::thread::sleep(std::time::Duration::from_nanos(10000000));
        }
    });*/
}
