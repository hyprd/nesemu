#![allow(warnings)]
mod bus;
mod cartridge;
mod cpu;
mod joypad;
mod opcodes;
mod ppu;
mod mapper;

use bus::Bus;
use cartridge::MirroringType;
use cartridge::Cartridge;
use cpu::Memory;
use cpu::CPU;
use ppu::frame::Frame;
use ppu::PPU;
use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::EventPump;
use std::collections::HashMap;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

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

fn handle_user_input(cpu: &mut CPU, event_pump: &mut EventPump) {
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
                keycode: Some(Keycode::S),
                ..
            } => {
                cpu.mem_write(0xff, 0x73);
            }
            Event::KeyDown {
                keycode: Some(Keycode::A),
                ..
            } => {
                cpu.mem_write(0xff, 0x61);
            }
            Event::KeyDown {
                keycode: Some(Keycode::D),
                ..
            } => {
                cpu.mem_write(0xff, 0x64);
            }
            _ => {}
        }
    }
}

fn main() {
    let palette: [(u8, u8, u8); 64] = [
        (0x80, 0x80, 0x80),
        (0x00, 0x3D, 0xA6),
        (0x00, 0x12, 0xB0),
        (0x44, 0x00, 0x96),
        (0xA1, 0x00, 0x5E),
        (0xC7, 0x00, 0x28),
        (0xBA, 0x06, 0x00),
        (0x8C, 0x17, 0x00),
        (0x5C, 0x2F, 0x00),
        (0x10, 0x45, 0x00),
        (0x05, 0x4A, 0x00),
        (0x00, 0x47, 0x2E),
        (0x00, 0x41, 0x66),
        (0x00, 0x00, 0x00),
        (0x05, 0x05, 0x05),
        (0x05, 0x05, 0x05),
        (0xC7, 0xC7, 0xC7),
        (0x00, 0x77, 0xFF),
        (0x21, 0x55, 0xFF),
        (0x82, 0x37, 0xFA),
        (0xEB, 0x2F, 0xB5),
        (0xFF, 0x29, 0x50),
        (0xFF, 0x22, 0x00),
        (0xD6, 0x32, 0x00),
        (0xC4, 0x62, 0x00),
        (0x35, 0x80, 0x00),
        (0x05, 0x8F, 0x00),
        (0x00, 0x8A, 0x55),
        (0x00, 0x99, 0xCC),
        (0x21, 0x21, 0x21),
        (0x09, 0x09, 0x09),
        (0x09, 0x09, 0x09),
        (0xFF, 0xFF, 0xFF),
        (0x0F, 0xD7, 0xFF),
        (0x69, 0xA2, 0xFF),
        (0xD4, 0x80, 0xFF),
        (0xFF, 0x45, 0xF3),
        (0xFF, 0x61, 0x8B),
        (0xFF, 0x88, 0x33),
        (0xFF, 0x9C, 0x12),
        (0xFA, 0xBC, 0x20),
        (0x9F, 0xE3, 0x0E),
        (0x2B, 0xF0, 0x35),
        (0x0C, 0xF0, 0xA4),
        (0x05, 0xFB, 0xFF),
        (0x5E, 0x5E, 0x5E),
        (0x0D, 0x0D, 0x0D),
        (0x0D, 0x0D, 0x0D),
        (0xFF, 0xFF, 0xFF),
        (0xA6, 0xFC, 0xFF),
        (0xB3, 0xEC, 0xFF),
        (0xDA, 0xAB, 0xEB),
        (0xFF, 0xA8, 0xF9),
        (0xFF, 0xAB, 0xB3),
        (0xFF, 0xD2, 0xB0),
        (0xFF, 0xEF, 0xA6),
        (0xFF, 0xF7, 0x9C),
        (0xD7, 0xE8, 0x95),
        (0xA6, 0xED, 0xAF),
        (0xA2, 0xF2, 0xDA),
        (0x99, 0xFF, 0xFC),
        (0xDD, 0xDD, 0xDD),
        (0x11, 0x11, 0x11),
        (0x11, 0x11, 0x11),
    ];
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("NESemu", (256.0 * 3.0) as u32, (240.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::RGB24, 256, 240)
        .unwrap();

    let bytes: Vec<u8> = std::fs::read("roms/mario.nes").unwrap();
    let rom = Cartridge::new(&bytes).unwrap();

    let mut frame = Frame::new();

    let mut keymap = HashMap::new();
    keymap.insert(Keycode::Down, joypad::JoypadButton::DOWN);
    keymap.insert(Keycode::Up, joypad::JoypadButton::UP);
    keymap.insert(Keycode::Right, joypad::JoypadButton::RIGHT);
    keymap.insert(Keycode::Left, joypad::JoypadButton::LEFT);
    keymap.insert(Keycode::Space, joypad::JoypadButton::SELECT);
    keymap.insert(Keycode::Return, joypad::JoypadButton::START);
    keymap.insert(Keycode::A, joypad::JoypadButton::A);
    keymap.insert(Keycode::S, joypad::JoypadButton::B);

    let bus = Bus::new(rom, move |ppu: &PPU, joypad: &mut joypad::Joypad| {
        // let palette = Frame::read_palette_from_file("palettes/nes.hex");
        Frame::render(ppu, &mut frame, palette.to_vec());
        texture.update(None, &frame.frame_data, 256 * 3).unwrap();
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
                    if let Some(key) = keymap.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_pressed(*key, true);
                    }
                }
                Event::KeyUp { keycode, .. } => {
                    if let Some(key) = keymap.get(&keycode.unwrap_or(Keycode::Ampersand)) {
                        joypad.set_pressed(*key, false);
                    }
                }
                _ => {}
            }
        }
    });
    let mut cpu = CPU::new(bus);
    cpu.reset();
    cpu.run();
}
