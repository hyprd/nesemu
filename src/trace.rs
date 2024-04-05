use crate::cpu::AddressingMode;
use crate::cpu::Memory;
use crate::cpu::CPU;
use crate::opcodes;
use std::collections::HashMap;

pub fn trace(cpu: &mut CPU) -> String {
    let ref opcodes: HashMap<u8, &'static opcodes::Opcode> = *opcodes::OPCODES_JMP_TABLE;
    let opcode = cpu.mem_read(cpu.reg_pc);
    let instruction = opcodes.get(&opcode).unwrap();
    let start = cpu.reg_pc;
    let mut hex_dump = vec![];
    hex_dump.push(opcode);

    let (address, address_value) = match instruction.addressing_mode {
        AddressingMode::IMM | AddressingMode::REL => (0, 0),
        _ => {
            cpu.reg_pc += 1; 
            let addr = cpu.resolve_addressing_mode(&instruction.addressing_mode);
            cpu.reg_pc -= 1;
            (addr, cpu.mem_read(addr))
        }
    };

    let tmp = match &instruction.length {
        1 => match &instruction.instruction {
            0x0A | 0x4A | 0x2A | 0x6A => format!("A "),
            _ => String::from(""),
        },
        2 => {
            let addr: u8 = cpu.mem_read(start + 1);
            hex_dump.push(addr);
            match &instruction.addressing_mode {
                AddressingMode::IMM => format!("#${:02x}", addr),
                AddressingMode::ZP => format!("${:02x} = {:02x}", address, address_value),
                AddressingMode::ZP_X => {
                    format!("${:02x},X @ {:02x} = {:02x}", addr, address, address_value)
                }
                AddressingMode::ZP_Y => {
                    format!("${:02x},Y @ {:02x} = {:02x}", addr, address, address_value)
                }
                AddressingMode::IND_X => format!(
                    "(${:02x},X) @ {:02x} = {:04x} = {:02x}",
                    addr,
                    addr.wrapping_add(cpu.reg_x),
                    address,
                    address_value
                ),
                AddressingMode::IND_Y => format!(
                    "(${:02x}),Y @ {:04x} = {:04x} = {:02x}",
                    addr,
                    addr.wrapping_sub(cpu.reg_y),
                    address,
                    address_value
                ),
                AddressingMode::REL => {
                    let addr: usize = (start as usize + 2).wrapping_add((addr as i8) as usize);
                    format!("${:04x}", addr)
                }
                _ => "".to_string(),
            }
        }
        3 => {
            let ll = cpu.mem_read(start + 1);
            let hh = cpu.mem_read(start + 2);
            hex_dump.push(ll);
            hex_dump.push(hh);
            let addr = cpu.mem_read_u16(start + 1);
            match &instruction.addressing_mode {
                AddressingMode::REL => {
                    if instruction.instruction == 0x6C {
                        let mut jump_address = 0;
                        if (addr & 0x00FF == 0x00FF) {
                            let lo = cpu.mem_read(address);
                            let hi = cpu.mem_read(address & 0xFF00);
                            jump_address = (hi as u16) << 8 | (lo as u16);
                        } else {
                            jump_address = cpu.mem_read_u16(address);
                        }
                        format!("(${:04x}) = {:04x}", address, jump_address)
                    } else {
                        format!("${:04x}", addr)
                    }
                }
                AddressingMode::ABS => format!("${:04x} = {:02x}", address, address_value),
                AddressingMode::ABS_X => {
                    format!("${:04x},X @ {:04x} = {:02x}", addr, address, address_value)
                }
                AddressingMode::ABS_Y => {
                    format!("${:04x},Y @ {:04x} = {:02x}", addr, address, address_value)
                }
                _ => panic!("Unexpected addressing mode",),
            }
        }
        _ => String::from(""),
    };
    let hex_str = hex_dump
        .iter()
        .map(|z| format!("{:02x}", z))
        .collect::<Vec<String>>()
        .join(" ");
    let asm_str = format!("{:04x}  {:8} {: >4} {}", start, hex_str, instruction.mnemonic, tmp)
        .trim()
        .to_string();

    format!(
        "{:47} A:{:02x} X:{:02x} Y:{:02x} P:{:02x} SP:{:02x}",
        asm_str, cpu.reg_a, cpu.reg_x, cpu.reg_y, cpu.reg_status, cpu.reg_sp,
    )
    .to_ascii_uppercase()
}
