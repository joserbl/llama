use cpu;
use cpu::Cpu;
use cpu::decoder_thumb as thumb;
use bitutils::sign_extend;

pub fn b_1(cpu: &mut Cpu, data: thumb::b_1::InstrDesc) -> cpu::InstrStatus {
    let offset_8 = bf!(data.signed_imm_8);
    let cond = bf!(data.cond);

    if !cpu::cond_passed(cond as u32, &cpu.cpsr) {
        return cpu::InstrStatus::InBlock;
    }

    let addr = (cpu.regs[15] as i32 + (sign_extend(offset_8 as u32, 8) << 1)) as u32;
    cpu.branch(addr);
    cpu::InstrStatus::Branched
}

pub fn branch(cpu: &mut Cpu, data: thumb::branch::InstrDesc) -> cpu::InstrStatus {
    let offset_11 = bf!(data.offset_11);

    match bf!(data.h_bits) {
        0b00 => {
            let addr = (cpu.regs[15] as i32 + (sign_extend(offset_11 as u32, 11) << 1)) as u32;
            cpu.branch(addr);
            cpu::InstrStatus::Branched
        },
        0b01 => {
            let addr = (cpu.regs[14] + (offset_11 << 1) as u32) & 0xFFFFFFFC;
            cpu.regs[14] = (cpu.regs[15] - 2) as u32 | 1;
            bf!((cpu.cpsr).thumb_bit = 0);
            cpu.branch(addr);
            cpu::InstrStatus::Branched
        },
        0b10 => {
            cpu.regs[14] = (cpu.regs[15] as i32 + (sign_extend(offset_11 as u32, 11) << 12)) as u32;
            cpu::InstrStatus::InBlock
        },
        0b11 => {
            let addr = cpu.regs[14] + (offset_11 << 1) as u32;
            cpu.regs[14] = (cpu.regs[15] - 2) as u32 | 1;
            cpu.branch(addr);
            cpu::InstrStatus::Branched
        },
        _ => unreachable!(),
    }
}

pub fn blx_2(cpu: &mut Cpu, data: thumb::blx_2::InstrDesc) -> cpu::InstrStatus {
    let rm = bf!(data.rm) | (bf!(data.h2) << 3);
    let addr = cpu.regs[rm as usize];

    cpu.regs[14] = (cpu.regs[15] - 2) as u32 | 1;
    bf!((cpu.cpsr).thumb_bit = bit!(addr, 0));

    cpu.branch(addr & 0xFFFFFFFE);
    cpu::InstrStatus::Branched
}

pub fn bx(cpu: &mut Cpu, data: thumb::bx::InstrDesc) -> cpu::InstrStatus {
    let addr = cpu.regs[((bf!(data.h2) << 3) | bf!(data.rm)) as usize];
    bf!((cpu.cpsr).thumb_bit = bit!(addr, 0));
    cpu.branch(addr & 0xFFFFFFFE);
    cpu::InstrStatus::Branched
}