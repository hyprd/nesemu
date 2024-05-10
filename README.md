# nesemu
NES emulator written in Rust.

It currently only supports games with an NROM mapper (namely Mario and PacMan) with more mapper support to come!.

### Building
Requires an Rust-SDL2 installation. Windows guide [here](https://github.com/Rust-SDL2/rust-sdl2?tab=readme-ov-file#windows-msvc).

### Todo
- Implement APU
- More precise PPU timing
- Add the last few illegal opcodes
- Extend mapper support
- Fix PPU bugs, mainly scrolling glitches
- Probably more!
