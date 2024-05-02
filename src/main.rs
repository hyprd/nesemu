#![allow(warnings)]
mod bus;
mod cartridge;
mod cpu;
mod joypad;
mod opcodes;
mod ppu;

use bus::Bus;
use cartridge::MirroringType;
use cartridge::ROM;
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

    let bytes: Vec<u8> = std::fs::read("roms/pacman.nes").unwrap();
    let rom = ROM::new(&bytes).unwrap();

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
        let palette = Frame::read_palette_from_file("palettes/nes.hex");
        Frame::render(ppu, &mut frame, palette);
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
