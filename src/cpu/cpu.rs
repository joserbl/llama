use cpu;
use ram;

// Program status register
create_bitfield!(Psr: u32, {
    mode: 0 => 4,
    thumb_bit: 5 => 5,
    disable_fiq_bit: 6 => 6,
    disable_irq_bit: 7 => 7,
    q_bit: 27 => 27,
    v_bit: 28 => 28,
    c_bit: 29 => 29,
    z_bit: 30 => 30,
    n_bit: 31 => 31
});

pub struct Cpu {
    pub regs: [u32; 16],
    pub cpsr: Psr::Type,
    pub spsr: [Psr::Type; 5],
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            regs: [0; 16],
            cpsr: Psr::new(0),
            spsr: [
                Psr::new(0), Psr::new(0), Psr::new(0),
                Psr::new(0), Psr::new(0)
            ],
        }
    }

    pub fn reset(&mut self, entry: u32) {
        self.regs[15] = entry + self.get_pc_offset();
        self.cpsr.set::<Psr::mode>(0b10011);
        self.cpsr.set::<Psr::thumb_bit>(0b0);
        self.cpsr.set::<Psr::disable_fiq_bit>(0b1);
        self.cpsr.set::<Psr::disable_irq_bit>(0b1);
    }

    pub fn get_pc_offset(&self) -> u32 {
        if self.cpsr.get::<Psr::thumb_bit>() == 1 {
            4
        } else {
            8
        }
    }

    pub fn get_current_spsr(&mut self) -> &mut Psr::Type {
        // TODO: Implement
        &mut self.spsr[0]
    }

    pub fn spsr_make_current(&mut self) {
        // TODO: Implement
    }

    #[inline(always)]
    pub fn branch(&mut self, addr: u32) {
        self.regs[15] = addr + self.get_pc_offset();
        // TODO: Invalidate pipeline once/if we have one
    }

    pub fn run(&mut self, mut ram: &mut ram::Ram) {
        loop {
            let addr = self.regs[15] - self.get_pc_offset();
            let encoding = ram.read::<u32>(addr);

            if self.cpsr.get::<Psr::thumb_bit>() == 0 {
                let instr = cpu::decode_arm_instruction(encoding);
                cpu::interpret_arm(self, ram, instr);
            } else {
                panic!("Thumb not supported!");
            }
        }
    }
}