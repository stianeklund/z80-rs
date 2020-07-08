use crate::instruction_info::{Instruction, Register, Register::*};
use crate::memory::{Memory, MemoryRW};

pub struct Cpu {
    pub current_instruction: String,
    pub opcode: u16,
    pub next_opcode: u16,
    pub breakpoint: bool,
    pub debug: bool,
    pub reg: Registers,
    pub flags: Flags,
    pub cycles: usize, // CPU T states
    pub io: Io,
    pub int: Interrupt,
    pub instruction: Instruction,
    pub int_pending: bool,
    pub cpm_compat: bool,
    pub memory: Memory,
}

#[derive(Default)]
pub struct Registers {
    // Main Registers
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    // Shadow registers
    pub a_: u8,
    pub b_: u8,
    pub c_: u8,
    pub d_: u8,
    pub e_: u8,
    pub h_: u8,
    pub l_: u8,

    // Alternate registers:
    pub m: u8,
    pub i: u8, // Interrupt vector
    pub r: u8, // Refresh counter
    pub pc: u16,
    pub prev_pc: u16,

    // Index Registers:
    pub sp: u16,
    pub ix: u16,
    pub iy: u16,
}

#[derive(Default)]
pub struct Io {
    pub port: u8,
    pub value: u8,
    pub input: bool,
    output: bool,
}

#[derive(Default, Debug)]
pub struct Flags {
    pub sf: bool, // Sign
    pub zf: bool, // Zero
    pub yf: bool, // Copy of bit 4 of the result
    pub hf: bool, // Half carry (AC)
    pub xf: bool, // Copy of bit 5 of the result
    pub pf: bool, // Parity
    pub nf: bool, // Subtract. Set if the last operation was a subtraction
    pub cf: bool, // Carry flag

    // Shadow
    pub sf_: bool,
    pub zf_: bool,
    pub yf_: bool,
    pub hf_: bool,
    pub xf_: bool,
    pub pf_: bool,
    pub nf_: bool,
    pub cf_: bool,
}

// IFF1 determines whether interrupts are allowed.
// IFF2's value is copied to PF by LD,AI and LD A, R
// When an NMI occurs IFF1 is reset, IFF2 is left unchanged.
// http://z80.info/z80info.htm (see f)
#[derive(Default, Debug)]
pub struct Interrupt {
    pub halt: bool, // Has the CPU halted?
    pub irq: bool,
    pub vector: u8,
    pub nmi_pending: bool,
    pub nmi: bool,
    pub int: bool,
    pub iff1: bool,
    pub iff2: bool,
    pub mode: u8,
}

impl Flags {
    fn new() -> Self {
        Self {
            sf: false,
            zf: false,
            yf: false,
            hf: false,
            xf: false,
            pf: false,
            nf: false,
            cf: false,
            sf_: false,
            zf_: false,
            yf_: false,
            hf_: false,
            xf_: false,
            pf_: false,
            nf_: false,
            cf_: false,
        }
    }
    // Creates a bit field from our CPU flags
    pub(crate) fn get(&self) -> u8 {
        let result: u8 = if self.sf { 0x80 } else { 0x0 }
            | if self.zf { 0x40 } else { 0x0 }
            | if self.yf { 0x20 } else { 0x0 }
            | if self.hf { 0x10 } else { 0x0 }
            | if self.xf { 0x08 } else { 0x0 }
            | if self.pf { 0x04 } else { 0x0 }
            | if self.nf { 0x02 } else { 0x0 }
            | if self.cf { 0x01 } else { 0x0 };
        result
    }
    pub fn set(&mut self, value: u8) {
        self.sf = (value & 0x80) != 0;
        self.zf = (value & 0x40) != 0;
        self.yf = (value & 0x20) != 0;
        self.hf = (value & 0x10) != 0;
        self.xf = (value & 0x08) != 0;
        self.pf = (value & 0x04) != 0;
        self.nf = (value & 0x02) != 0;
        self.cf = (value & 0x01) != 0;
    }

    pub(crate) fn get_shadow(&self) -> u8 {
        let shadow: u8 = if self.sf_ { 0x80 } else { 0x0 }
            | if self.zf_ { 0x40 } else { 0x0 }
            | if self.yf_ { 0x20 } else { 0x0 }
            | if self.hf_ { 0x10 } else { 0x0 }
            | if self.xf_ { 0x08 } else { 0x0 }
            | if self.pf_ { 0x04 } else { 0x0 }
            | if self.nf_ { 0x02 } else { 0x0 }
            | if self.cf_ { 0x01 } else { 0x0 };
        shadow
    }

    pub fn set_shadow(&mut self, value: u8) {
        self.sf_ = (value & 0x80) != 0;
        self.zf_ = (value & 0x40) != 0;
        self.yf_ = (value & 0x20) != 0;
        self.hf_ = (value & 0x10) != 0;
        self.xf_ = (value & 0x08) != 0;
        self.pf_ = (value & 0x04) != 0;
        self.nf_ = (value & 0x02) != 0;
        self.cf_ = (value & 0x01) != 0;
    }

    fn swap(&mut self) {
        let f = self.get();
        self.set(self.get_shadow());
        self.set_shadow(f);
    }
}

impl MemoryRW for Cpu {
    #[inline]
    fn read8(&self, addr: u16) -> u8 {
        if self.cpm_compat {
            self.memory[addr]
        } else if addr < 0x4000 {
            self.memory.rom[addr as usize]
        } else if addr == 0x5000 {
            self.int.int as u8
        } else if addr < 0x5000 {
            self.memory.ram[addr as usize - 0x4000]
        } else {
            self.memory.rom[addr as usize]
        }
    }

    #[inline]
    fn read16(&self, addr: u16) -> u16 {
        u16::from_le_bytes([self.read8(addr), self.read8(addr + 1)])
    }

    #[inline]
    fn write16(&mut self, addr: u16, word: u16) {
        self.write8(addr, word as u8);
        self.write8(addr.wrapping_add(1), (word >> 8) as u8);
    }

    #[inline]
    fn write8(&mut self, addr: u16, byte: u8) {
        if self.cpm_compat {
            self.memory[addr] = byte;
        } else if !self.cpm_compat && addr < 0x4000 {
            self.memory.ram[addr as usize] = byte;
        } else if !self.cpm_compat && addr < 0x5000 {
            self.memory.ram[addr as usize - 0x4000] = byte;
        } else if addr == 0x5000 {
            self.int_pending = true;
        } else {
            self.memory.ram[addr as usize] = byte;
        }
    }
}

impl Cpu {
    pub fn default() -> Self {
        Self {
            opcode: 0,
            next_opcode: 0,
            reg: Registers::default(),
            flags: Flags::new(),
            cycles: 0,
            current_instruction: String::new(),
            debug: false,
            breakpoint: false,
            io: Io::default(),
            int: Interrupt::default(),
            int_pending: false,
            instruction: Instruction::default(),
            memory: Memory::default(),
            cpm_compat: false,
        }
    }

    fn read_reg(&self, reg: Register) -> u8 {
        match reg {
            A => self.reg.a,
            B => self.reg.b,
            C => self.reg.c,
            D => self.reg.d,
            E => self.reg.e,
            H => self.reg.h,
            L => self.reg.l,
            M => self.reg.m,
            I => self.reg.i,
            R => self.reg.r,
            IXH => (self.reg.ix >> 8) as u8,
            IXL => (self.reg.ix & 0xFF) as u8,
            IYH => (self.reg.iy >> 8) as u8,
            IYL => (self.reg.iy & 0xFF) as u8,

            // We only use HL here indexed in memory anyways..
            HL => self.read8(self.read_pair(HL)),
            IxIm => {
                let byte = self.read8(self.reg.pc.wrapping_add(1)) as i8;
                self.read8(self.reg.ix.wrapping_add(byte as u16))
            }
            IyIm => {
                let byte = self.read8(self.reg.pc.wrapping_add(1)) as i8;
                self.read8(self.reg.iy.wrapping_add(byte as u16))
            }
            _ => {
                println!(
                    "Called by:{}, Opcode:{:02X}",
                    self.current_instruction, self.opcode
                );
                eprintln!("Instruction:{:?}", Instruction::decode(self));
                eprintln!("{:#?}", Instruction::print_disassembly(self));
                panic!("Register not supported:{:#?}", reg)
            }
        }
    }

    fn write_reg(&mut self, reg: Register, value: u8) {
        match reg {
            A => self.reg.a = value,
            B => self.reg.b = value,
            C => self.reg.c = value,
            D => self.reg.d = value,
            E => self.reg.e = value,
            H => self.reg.h = value,
            L => self.reg.l = value,
            M => self.reg.m = value,
            I => self.reg.i = value,
            R => self.reg.r = value,
            HL => self.write8(self.read_pair(HL), value), // HL is only used indexed to memory
            IXH => self.reg.ix = (self.reg.ix & 0x00FF) | ((value as u16) << 8),
            IXL => self.reg.ix = (self.reg.ix & 0xFF00)   | value as u16,
            IYH => self.reg.iy = (self.reg.iy & 0x00FF) | ((value as u16) << 8) as u16,
            IYL => self.reg.iy = (self.reg.iy & 0xFF00)   | value as u16,
            IxIm => self.write8(self.reg.ix + self.read8(self.reg.pc + 1) as u16, value),
            IyIm => self.write8(self.reg.iy + self.read8(self.reg.pc + 1) as u16, value),
            _ => panic!(format!(
                "Writing to RP: {:#?}, is not supported by write_reg, called by: {}, opcode:{:02X}{:02X}",
                reg, self.current_instruction, self.opcode, self.next_opcode
            )),
        }
    }

    // Loads register pair with direct value
    pub fn write_pair(&mut self, reg: Register, value: u16) {
        match reg {
            AF => {
                self.flags.set((value & 0xFF) as u8);
                self.reg.a = (value >> 8) as u8;
            }
            DE => {
                self.reg.d = (value >> 8) as u8;
                self.reg.e = (value & 0xFF) as u8;
            }
            BC => {
                self.reg.b = (value >> 8) as u8;
                self.reg.c = (value & 0xFF) as u8;
            }
            HL => {
                self.reg.h = (value >> 8) as u8;
                self.reg.l = (value & 0xFF) as u8;
            }
            IX => self.reg.ix = value,
            IY => self.reg.iy = value,
            SP => self.reg.sp = value,
            _ => panic!("Attempting to write to a non register pair: {:#?}", reg),
        }
    }

    #[inline]
    pub fn read_pair(&self, reg: Register) -> u16 {
        match reg {
            BC => (self.reg.b as u16) << 8 | (self.reg.c as u16),
            DE => (self.reg.d as u16) << 8 | (self.reg.e as u16),
            HL => (self.reg.h as u16) << 8 | (self.reg.l as u16),
            IX => self.reg.ix,
            IY => self.reg.iy,
            SP => self.reg.sp,
            AF => ((self.reg.a as u16) << 8 | (self.flags.get() as u16)),
            _ => panic!(
                "read_pair() called with reg:{:#?}, opcode:{:02X}{:02X}",
                reg, self.opcode, self.next_opcode
            ),
        }
    }

    #[inline]
    fn adv_pc(&mut self, t: u16) {
        self.reg.prev_pc = self.reg.pc;
        self.reg.pc = self.reg.pc.wrapping_add(t);
    }

    #[inline]
    fn adv_cycles(&mut self, t: usize) {
        self.cycles = self.cycles.wrapping_add(t);
    }

    // TODO refactor ADD / ADC instructions
    // pass value in from the caller and have one method for most of these
    fn adc(&mut self, reg: Register) {
        if reg == IxIm || reg == IyIm {
            self.adv_pc(2);
            self.adv_cycles(15);
        }
        let value = self.read_reg(reg) as u16;
        if reg == Register::HL {
            self.adv_cycles(3);
        }
        let result: u16 = (self.reg.a as u16)
            .wrapping_add(value as u16)
            .wrapping_add(self.flags.cf as u16);

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = self.hf_add(self.reg.a, value as u8);
        self.flags.pf = self.overflow(self.reg.a as i8, value as i8, result as i8);
        self.flags.nf = false;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.cf = (result & 0x0100) != 0;

        self.reg.a = result as u8;

        if reg == IXH || reg == IXL || reg == IYL || reg == IYH {
            self.adv_pc(1);
            self.adv_cycles(4);
        }

        self.adv_cycles(4);
        self.adv_pc(1);
    }
    fn adc_hl(&mut self, reg: Register) {
        let hl = self.read_pair(HL);
        let (result, value) = (
            (self.read_pair(HL) as u32)
                .wrapping_add(self.read_pair(reg) as u32)
                .wrapping_add(self.flags.cf as u32),
            self.read_pair(reg),
        );

        self.flags.sf = (result & 0x8000) != 0;
        self.flags.zf = (result & 0xFFFF) == 0;
        self.flags.hf = self.hf_add_w(hl, value as u16, true);
        self.flags.pf =
            (hl & 0x8000) == (value & 0x8000) && (hl & 0x8000) != ((result & 0x8000) as u16);
        self.flags.yf = (result & 0x2000) != 0;
        self.flags.xf = (result & 0x0800) != 0;
        self.flags.cf = (result & 0x10000) != 0;
        self.flags.nf = false;

        self.write_pair(HL, result as u16);

        self.adv_cycles(15);
        self.adv_pc(2);
    }

    // Add Immediate to Accumulator with Carry
    fn adc_im(&mut self) {
        let value = self.read8(self.reg.pc + 1) as u16;

        // Add immediate with accumulator + carry flag value
        let carry = self.flags.cf as u8;
        let result = (value)
            .wrapping_add(self.reg.a as u16)
            .wrapping_add(carry as u16);

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = self.hf_add(self.reg.a, value as u8);
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.nf = false;
        self.flags.pf = self.overflow(self.reg.a as i8, value as i8, result as i8);
        self.flags.cf = (result & 0x0100) != 0;

        self.reg.a = result as u8;

        self.adv_cycles(7);
        self.adv_pc(2);
    }

    pub(crate) fn add_hl(&mut self, reg: Register) {
        let hl: u16 = self.read_pair(HL);
        let (result, add) = (
            (self.read_pair(HL) as u32).wrapping_add(self.read_pair(reg) as u32),
            self.read_pair(reg),
        );
        self.write_pair(HL, result as u16);

        self.flags.cf = ((result >> 8) & 0x0100) != 0;
        self.flags.hf = self.hf_add_w(hl, add as u16, false);
        self.flags.nf = false;
        self.flags.yf = ((result >> 8) & 0x20) != 0;
        self.flags.xf = ((result >> 8) & 0x08) != 0;
        self.adv_cycles(11);
        self.adv_pc(1);
    }
    // Passes ADD IX & ADD IY Zexdoc tests
    pub(crate) fn add_rp(&mut self, dst: Register, src: Register) {
        let (result, add) = (
            (self.read_pair(dst) as u32).wrapping_add(self.read_pair(src) as u32),
            self.read_pair(src),
        );
        self.write_pair(dst, result as u16);

        self.flags.cf = ((result >> 8) & 0x0100) != 0;
        self.flags.hf = self.hf_add_w(self.read_pair(HL), add as u16, false);
        self.flags.nf = false;
        self.flags.yf = ((result >> 8) & 0x20) != 0;
        self.flags.xf = ((result >> 8) & 0x08) != 0;
        self.adv_cycles(15);
        self.adv_pc(2);
    }

    // Can be consolidated into just simply using addressing modes..

    fn add(&mut self, reg: Register) {
        let value = self.read_reg(reg) as u16;
        if reg == HL {
            self.adv_cycles(3);
        }
        if reg == IxIm || reg == IyIm {
            self.adv_pc(2);
            self.adv_cycles(15);
        }
        if reg == IXL || reg == IXH || reg == IYL || reg == IYL {
            self.adv_cycles(4);
            self.adv_pc(1);
        }
        let result = (self.reg.a as u16).wrapping_add(value as u16);

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = self.hf_add(self.reg.a, value as u8);
        self.flags.pf = self.overflow(self.reg.a as i8, value as i8, result as i8);
        self.flags.nf = false;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.cf = (result & 0x0100) != 0;

        self.reg.a = result as u8;

        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // Add Immediate to Accumulator
    fn adi(&mut self) {
        // Read next byte of immediate data (low).
        let value = self.read8(self.reg.pc + 1) as u16;
        let result = (self.reg.a as u16).wrapping_add(value as u16);

        // Set CPU flags with new accumulator values
        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.pf = self.overflow(self.reg.a as i8, value as i8, result as i8);
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.nf = false;
        self.flags.hf = self.hf_add(self.reg.a, value as u8);
        self.flags.cf = (result & 0x0100) != 0;

        self.reg.a = result as u8;

        self.adv_cycles(7);
        self.adv_pc(2);
    }

    pub fn and(&mut self, reg: Register) {
        // TODO Clean up
        let value = self.read_reg(reg) as u16;
        if reg == IyIm || reg == IxIm {
            self.adv_pc(2);
            self.adv_cycles(15);
        } else if reg == HL {
            self.adv_cycles(3);
        }
        if reg == IXL || reg == IXH || reg == IYL || reg == IYH {
            self.adv_cycles(4);
            self.adv_pc(1);
        }
        // And value with accumulator
        let result = self.reg.a & value as u8;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = result == 0;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.nf = false;
        self.flags.hf = true;
        self.flags.pf = self.parity(result as u8);
        self.flags.cf = false;

        self.reg.a = result as u8;

        self.adv_cycles(4);
        self.adv_pc(1);
    }

    fn ani(&mut self) {
        // The byte of immediate data is ANDed with the contents of the accumulator
        let value = self.read8(self.reg.pc + 1);
        let result = self.reg.a as u16 & value as u16;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.nf = false;
        self.flags.hf = true;
        self.flags.pf = self.parity(result as u8);
        self.flags.cf = false;

        self.reg.a = result as u8;

        self.adv_cycles(7);
        self.adv_pc(2);
    }
    // 0xCB Extended Opcode Bit instructions
    fn bit(&mut self, bit: u8, reg: Register) {
        let result = self.read_reg(reg) & (1 << bit);

        // Test bit n of register
        if reg == HL {
            self.adv_cycles(4);
        };
        if reg == IxIm || reg == IyIm {
            self.adv_pc(2);
            self.adv_cycles(12);
        }

        // P/V is set to the same value as Z .
        // S is reset unless the instruction is BIT 7, r, and bit 7 of r is set.
        // Match towards DDCBnn
        match self.read8(self.reg.pc + 1) {
            0x78..=0x7D => {
                if self.reg.r & (1 << 7) != 0 {
                    self.flags.sf = true;
                }
            }
            _ => self.flags.sf = (result & 0x80) != 0,
        }
        self.flags.zf = result == 0;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.nf = false;
        self.flags.hf = true;
        self.flags.pf = self.flags.zf; // TODO: Double check this
        self.adv_pc(2);
        self.adv_cycles(8);
    }

    fn set(&mut self, bit: u8, reg: Register) {
        self.write_reg(reg, self.read_reg(reg) | (1 << bit));

        if reg == IxIm || reg == IyIm {
            self.adv_pc(2);
            self.adv_cycles(15);
        }
        self.adv_pc(2);
        self.adv_cycles(8);
    }
    fn res(&mut self, bit: u8, reg: Register) {
        self.write_reg(reg, self.read_reg(reg) & (1 << bit));
        if reg == IxIm || reg == IyIm {
            self.adv_pc(2);
            self.adv_cycles(15);
        }
        self.adv_pc(2);
        self.adv_cycles(8);
    }
    fn djnz(&mut self) {
        // The b register is decremented, and if not zero the signed value * is added to PC
        // The jump is measured from the start of the last instruction opcode
        self.adv_cycles(1);
        self.reg.b = self.reg.b.wrapping_sub(1);
        self.jr_cond(self.reg.b != 0);
    }
    fn jr(&mut self, offset: i16) {
        self.adv_pc(2);
        self.reg.prev_pc = self.reg.pc;
        self.reg.pc = (self.reg.pc as i16 + offset) as u16;
        self.adv_cycles(12);
    }
    // "Generic" function for conditional JR operations
    fn jr_cond(&mut self, cond: bool) {
        // E.g if zero flag == 0 { JR + offset
        let byte = self.read8(self.reg.pc + 1) as i8;
        if cond {
            self.jr(byte as i16);
        } else {
            self.adv_cycles(7);
            self.adv_pc(2);
        }
    }
    fn jp(&mut self, addr: u16, additional_cycles: usize) {
        self.reg.prev_pc = self.reg.pc;
        self.adv_cycles(additional_cycles);
        self.reg.pc = addr;
    }
    fn jp_cond(&mut self, cond: bool) {
        if cond {
            self.reg.prev_pc = self.reg.pc;
            self.reg.pc = self.read16(self.reg.pc + 1);
        } else {
            self.adv_pc(3);
        }
        self.adv_cycles(10);
    }

    // Jump to address in H:L
    fn pchl(&mut self) {
        self.adv_cycles(4);
        self.reg.prev_pc = self.reg.pc;
        self.reg.pc = self.read_pair(Register::HL) as u16;
    }

    #[inline]
    fn ld(&mut self, dst: Register, src: Register) {
        let mut value: u16 = match src {
            A | B | C | D | E | H | L | I | R => u16::from(self.read_reg(src)),
            IXL | IYL | IXH | IYH => {
                self.adv_cycles(4);
                self.adv_pc(1);
                u16::from(self.read_reg(src))
            }
            IxIm | IyIm => self.read_reg(src) as u16,
            BC | DE | HL => self.read_pair(src),
            _ => panic!("Non handled LD source"),
        };

        match dst {
            A | B | C | D | E | H | L => {
                if src == HL || src == BC || src == DE {
                    // LD r, (HL) / (BC) etc
                    value = self.read8(self.read_pair(src)) as u16;
                    self.adv_cycles(3);
                } else if src == IxIm || src == IyIm {
                    value = self.read_reg(src) as u16;
                    self.adv_pc(2);
                    self.adv_cycles(15);
                } else if (src == R) || (src == I) {
                    self.flags.sf = (self.reg.a & 0x80) != 0;
                    self.flags.zf = self.reg.a == 0;
                    self.flags.pf = self.int.iff2;
                    self.flags.hf = false;
                    self.flags.nf = false;
                    self.adv_cycles(5);
                    self.adv_pc(1);
                }
                self.write_reg(dst, value as u8);
            }

            HL | BC | DE => {
                // LD (HL), r. LD (BC), r
                self.write8(self.read_pair(dst), self.read_reg(src));
                self.adv_cycles(3);
            }
            I | R => {
                self.adv_cycles(5);
                self.adv_pc(1);
                self.write_reg(dst, value as u8);
            }
            IXL | IYL | IYH | IXH => {
                self.write_reg(dst, value as u8);
                self.adv_pc(1);
                self.adv_cycles(4);
            }
            IxIm | IyIm => {
                self.write_reg(dst, value as u8);
                self.adv_pc(2);
                self.adv_cycles(15);
            }
            _ => panic!("Unhandled LD register"),
        }
        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // Transfers a byte of data from the memory location pointed to by hl to the memory location
    // pointed to by DE
    // Then HL and DE are incremented and BC decremented.
    fn ldi(&mut self) {
        // YF and XF are copies of bit 1 of n and bit 3 of n respectively.
        let hl = self.read8(self.read_pair(HL));
        self.write8(self.read_pair(DE), hl as u8);
        let n = hl.wrapping_add(self.reg.a);

        self.write_pair(HL, self.read_pair(HL).wrapping_add(1));
        self.write_pair(DE, self.read_pair(DE).wrapping_add(1));
        self.write_pair(BC, self.read_pair(BC).wrapping_sub(1));

        self.flags.pf = self.read_pair(BC) != 0;
        self.flags.nf = false;
        self.flags.nf = false;
        self.flags.yf = (n & 0x02) != 0;
        self.flags.xf = (n & 0x08) != 0;
        self.adv_cycles(16);
        self.adv_pc(2);
    }

    // 0xEDB0 Extended instruction
    fn ldir(&mut self) {
        // LDIR is basically LDI + BC if BC is not 0 decrease PC by 2.
        self.ldi();
        if self.read_pair(BC) != 0 {
            self.reg.prev_pc = self.reg.pc;
            self.reg.pc = self.reg.pc.wrapping_sub(2);
            self.adv_cycles(5);
        }
        if self.read_pair(BC) <= 0 {
            self.reg.r = (self.reg.r & 0x80) | (self.reg.r.wrapping_add(0) as u8 & 0x7f);
        }
    }

    // Extended instructions: ex: LD (**), HL
    // 0xED63, 0xED53 etc..
    // Stores (REGPAIR) into the memory loc pointed to by **
    // TODO & LOAD INDIRECT BUG?
    fn ld_mem_nn_rp(&mut self, reg: Register) {
        let ptr = self.read16(self.reg.pc + 2);
        self.write16(ptr, self.read_pair(reg));
        self.adv_cycles(20);
        self.adv_pc(4);
    }

    // Extended instructions: ex: LD HL, (**)
    // 0xED6B, 0xED5B etc..
    // Loads the value pointed to by ** into (REGPAIR)
    fn ld_rp_mem_nn(&mut self, reg: Register) {
        let word = self.read16(self.reg.pc + 2);
        self.write_pair(reg, self.read16(word));
        self.adv_cycles(20);
        self.adv_pc(4);
    }

    // Load Register Pair Immediate
    // LXI H, 2000H (2000H is stored in HL & acts as as memory pointer)
    #[inline]
    fn ld_rp_nn(&mut self, reg: Register) {
        if reg == IX || reg == IY {
            self.adv_cycles(4);
            self.adv_pc(1);
        }
        self.write_pair(reg, self.read16(self.reg.pc + 1));

        self.adv_cycles(10);
        self.adv_pc(3);
    }

    // LD **, A
    // Store Accumulator direct
    fn ld_nn_r(&mut self) {
        let imm = self.read16(self.reg.pc + 1);
        self.write8(imm, self.reg.a);
        self.adv_cycles(13);
        self.adv_pc(3);
    }

    #[inline]
    fn call(&mut self, addr: u16) {
        let ret: u16 = self.reg.pc.wrapping_add(3);
        self.reg.prev_pc = self.reg.pc;
        self.memory[self.reg.sp.wrapping_sub(1)] = (ret >> 8) as u8;
        // Low order byte
        self.memory[self.reg.sp.wrapping_sub(2)] = ret as u8;
        // Push return address to stack
        self.reg.sp = self.reg.sp.wrapping_sub(2);
        match addr {
            0xCC | 0xCD | 0xC4 | 0xD4 | 0xDC | 0xE4 | 0xEC | 0xF4 | 0xFC | 0x66 => {
                self.reg.pc = self.read16(self.reg.pc + 1);
            }
            _ => {
                // println!("CALL to address:{:04X}", addr);
                self.reg.pc = addr;
            }
        };
        self.adv_cycles(17);
    }

    // Conditional calls
    fn call_cond(&mut self, addr: u16, cond: bool) {
        if cond {
            self.call(addr);
        } else {
            self.adv_cycles(10);
            self.adv_pc(3);
        }
    }

    fn cpl(&mut self) {
        self.reg.a ^= 0xFF;
        self.flags.hf = true;
        self.flags.nf = true;
        self.flags.yf = (self.reg.a & 0x20) != 0;
        self.flags.xf = (self.reg.a & 0x08) != 0;
        self.adv_cycles(4);
        self.adv_pc(1);
    }

    fn ccf(&mut self) {
        self.flags.hf = self.flags.cf;
        self.flags.cf = !self.flags.cf;
        self.flags.yf = (self.reg.a & 0x20) != 0;
        self.flags.xf = (self.reg.a & 0x08) != 0;
        self.flags.nf = false;
        self.adv_cycles(4);
        self.adv_pc(1);
    }
    fn cmp(&mut self, reg: Register) {
        let value = if reg == IxIm || reg == IyIm {
            self.adv_cycles(15);
            self.adv_pc(2);
            self.read_reg(reg) as u16
        } else if reg == HL {
            self.adv_cycles(3);
            self.memory[self.read_pair(HL)] as u16
        } else {
            self.read_reg(reg) as u16
        };
        if reg == IXL || reg == IXH || reg == IYL || reg == IYL {
            self.adv_cycles(4);
            self.adv_pc(1);
        }
        let result = (self.reg.a as u16).wrapping_sub(value as u16);
        let overflow = (self.reg.a as i8).overflowing_sub(value as i8).1;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = self.hf_sub(self.reg.a, value as u8);
        self.flags.nf = true;
        // The XF & YF flags use the non compared value
        self.flags.yf = (value & 0x20) != 0;
        self.flags.xf = (value & 0x08) != 0;
        self.flags.pf = overflow;
        self.flags.cf = (result & 0x0100) != 0;

        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // TODO Use addressing modes here
    // Compare Immediate with Accumulator
    fn cp_im(&mut self) {
        let value = self.read8(self.reg.pc + 1);
        let result = (self.reg.a as i16).wrapping_sub(value as i16);
        let overflow = (self.reg.a as i8).overflowing_sub(value as i8).1;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.yf = (value & 0x20) != 0;
        self.flags.hf = self.hf_sub(self.reg.a, value as u8);
        self.flags.xf = (value & 0x08) != 0;
        self.flags.pf = overflow;
        self.flags.nf = true;
        self.flags.cf = (result & 0x0100) != 0;

        self.adv_cycles(7);
        self.adv_pc(2);
    }
    // Extended instruction
    fn cpi(&mut self) {
        // TODO
        // Compares the value of the memory location pointed to by HL with A.
        // HL is then incremented and BC is decremented.
        // S,Z,H from (A - (HL) ) as in CP (HL)
        // F3 is bit 3 of (A - (HL) - H), H as in F after instruction
        // F5 is bit 1 of (A - (HL) - H), H as in F after instruction
        let value = self.read8(self.read_pair(HL));
        let result = (self.reg.a as u16).wrapping_sub(value as u16);
        let overflow = (self.reg.a as i8).overflowing_sub(value as i8).1;
        self.write_pair(HL, self.read_pair(HL).wrapping_add(1));
        self.write_pair(BC, self.read_pair(BC).wrapping_sub(1));

        self.flags.nf = true;
        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = self.hf_sub(self.reg.a, value);
        // self.flags.pf = self.overflow(value, result as u8);
        self.flags.pf = overflow;
        // self.flags.cf = (result & 0x0100) != 0;
        self.flags.yf = (value & 0x20) != 0;
        self.flags.xf = (value & 0x08) != 0;
        self.adv_pc(2);
        self.adv_cycles(16);
    }
    fn cpir(&mut self) {
        self.cpi();
        if self.read_pair(BC) != 0 && !self.flags.zf {
            self.reg.prev_pc = self.reg.pc;
            self.reg.pc = self.reg.pc.wrapping_sub(2);
            self.adv_cycles(5);
        }
        if self.read_pair(BC) <= 0 {
            self.reg.r = (self.reg.r & 0x80) | (self.reg.r.wrapping_add(0) as u8 & 0x7f);
        }
    }
    // Extended instruction
    fn cpd(&mut self) {
        // Same as CPI but HL is also decremented
        self.cpi();
        self.write_pair(HL, self.read_pair(HL).wrapping_sub(2))
    }

    fn cpdr(&mut self) {
        self.cpd();
        if self.read_pair(BC) != 0 && !self.flags.zf {
            self.reg.prev_pc = self.reg.pc;
            self.reg.pc = self.reg.pc.wrapping_sub(2);
            self.adv_cycles(5);
        }
        if self.read_pair(BC) <= 0 {
            self.reg.r = (self.reg.r & 0x80) | (self.reg.r.wrapping_add(0) as u8 & 0x7f);
        }
    }
    // Decrement memory or register
    fn dec(&mut self, reg: Register) {
        let result: u16 = match reg {
            A | B | C | D | E | H | L | HL | IXH | IXL | IYH | IYL | IxIm | IyIm => {
                self.write_reg(reg, self.read_reg(reg).wrapping_sub(1));
                self.read_reg(reg) as u16
            }
            _ => panic!("DEC on unsupported register: {:#?}", reg),
        };
        match reg {
            HL => self.adv_cycles(5),
            IxIm | IyIm => {
                self.adv_cycles(19);
                self.adv_pc(2);
            }
            IXH | IXL | IYH | IYL => {
                self.adv_pc(1);
                self.adv_cycles(4);
            }
            _ => {}
        }

        let overflow = (result as i8).wrapping_add(1).overflowing_sub(1).1;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = result == 0;
        self.flags.hf = self.hf_sub((result as u8).wrapping_add(1), 1);
        self.flags.pf = overflow;
        self.flags.nf = true;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;

        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // DEC register pair.
    fn dec_rp(&mut self, pair: Register) {
        self.write_pair(pair, self.read_pair(pair).wrapping_sub(1));
        if pair == IX || pair == IY {
            self.adv_cycles(4);
            self.adv_pc(1);
        }

        self.adv_cycles(6);
        self.adv_pc(1);
    }

    // Double precision add
    fn daa(&mut self) {
        let mut offset = 0;

        if self.flags.hf || self.reg.a & 0x0F > 0x09 {
            offset += 0x06;
        }
        if (self.reg.a > 0x99) || self.flags.cf {
            offset += 0x60;
            self.flags.cf = true;
        }
        if self.flags.nf {
            self.flags.hf = self.flags.hf && (self.reg.a & 0x0F) < 0x06;
            self.reg.a = self.reg.a.wrapping_sub(offset);
        } else {
            self.flags.hf = (self.reg.a & 0x0F) > 0x09;
            self.reg.a = self.reg.a.wrapping_add(offset);
        }
        let result = (self.reg.a as u16).wrapping_add(offset as u16);

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.pf = self.parity(result as u8);
        self.flags.yf = (self.reg.a & 0x20) != 0;
        self.flags.xf = (self.reg.a & 0x08) != 0;
        self.reg.a = result as u8;

        self.adv_cycles(4);
        self.adv_pc(1);
    }

    fn set_interrupt_mode(&mut self, mode: u8) {
        // println!("Setting interrupt mode 2");
        self.int.mode = mode;
        self.adv_cycles(8);
        self.adv_pc(2);
    }
    // EI & DI instructions
    fn interrupt(&mut self, value: bool) {
        self.int.int = value;
        if value {
            self.int.irq = true;
        } else if !value {
            self.int.iff1 = false;
            self.int.iff2 = false;
        }
        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // Rotate Accumulator Left Through Carry
    fn rla(&mut self) {
        // The contents of the accumulator are rotated one bit position to the left.
        // The high-order bit of the accumulator replaces the carry bit while the carry bit
        // replaces the high-order bit of the accumulator
        let carry = (self.reg.a >> 7) != 0;
        self.reg.a = (self.reg.a << 1) | ((self.flags.cf as u8) << 7);
        self.flags.nf = false;
        self.flags.hf = false;
        self.flags.yf = (self.reg.a & 0x20) != 0;
        self.flags.xf = (self.reg.a & 0x08) != 0;
        self.flags.cf = carry;

        self.adv_cycles(4);
        self.adv_pc(1);
    }

    fn rrc(&mut self, reg: Register) {
        self.write_reg(
            reg,
            (self.read_reg(reg) >> 1) | ((self.flags.cf as u8) << 7),
        );
        let value = self.read_reg(reg);

        self.flags.nf = false;
        self.flags.hf = false;
        self.flags.yf = (value & 0x20) != 0;
        self.flags.xf = (value & 0x08) != 0;
        self.flags.cf = (value & 0x80) != 0;
        self.parity(value);
        self.adv_pc(2);
        self.adv_cycles(8);
    }
    // Extended instruction 0xCB03
    fn rlc(&mut self, reg: Register) {
        let value = match reg {
            HL => {
                self.write_pair(
                    reg,
                    (self.read_pair(reg) << 1) | ((self.flags.cf as u8) & 1) as u16,
                );
                self.adv_cycles(7);
                self.read_pair(reg)
            }
            A | B | C | D | E | H | L => {
                self.write_reg(reg, (self.read_reg(reg) << 1) | ((self.flags.cf as u8) & 1));
                self.read_reg(reg) as u16
            }
            _ => unimplemented!("RLC on reg:{:#?}", reg),
        };

        self.flags.nf = false;
        self.flags.hf = false;
        self.flags.yf = (value & 0x20) != 0;
        self.flags.xf = (value & 0x08) != 0;
        self.flags.cf = (value & 0x80) != 0;
        self.parity(value as u8);
        self.adv_pc(2);
        self.adv_cycles(8);
    }

    fn rlc_ex(&mut self, src: Register, dst: Register) {
        if src == IxIm || src == IyIm {
            let value = match dst {
                A | B | C | D | E | H | L => {
                    self.write_reg(dst, (self.read_reg(src) << 1) | ((self.flags.cf as u8) & 1));
                    self.read_reg(dst) as u16
                }
                _ => unimplemented!("RLC on reg:{:#?}", dst),
            };

            self.flags.nf = false;
            self.flags.hf = false;
            self.flags.yf = (value & 0x20) != 0;
            self.flags.xf = (value & 0x08) != 0;
            self.flags.cf = (value & 0x80) != 0;
            self.parity(value as u8);
            self.adv_pc(4);
            self.adv_cycles(23);
        }
    }
    // Rotate Accumulator Right Through Carry
    fn rra(&mut self) {
        let carry = (self.reg.a & 1) != 0;
        self.reg.a = (self.reg.a >> 1) | ((self.flags.cf as u8) << 7);
        self.flags.cf = carry;
        self.flags.yf = (self.reg.a & 0x20) != 0;
        self.flags.xf = (self.reg.a & 0x08) != 0;
        self.flags.nf = false;
        self.flags.hf = false;
        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // Rotate Accumulator Left
    fn rlca(&mut self) {
        self.flags.cf = (self.reg.a >> 7) != 0;
        self.reg.a = (self.reg.a << 1) | self.flags.cf as u8;
        self.flags.yf = (self.reg.a & 0x20) != 0;
        self.flags.xf = (self.reg.a & 0x08) != 0;
        self.flags.nf = false;
        self.flags.hf = false;
        self.adv_cycles(4);
        self.adv_pc(1);
    }

    fn rrca(&mut self) {
        self.flags.cf = (self.reg.a & 1) != 0;
        self.reg.a = (self.reg.a >> 1) | ((self.flags.cf as u8) << 7);
        self.flags.yf = (self.reg.a & 0x20) != 0;
        self.flags.xf = (self.reg.a & 0x08) != 0;
        self.flags.nf = false;
        self.flags.hf = false;
        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // Conditional return
    fn ret_cond(&mut self, cond: bool) {
        if cond {
            self.adv_cycles(1);
            self.ret();
        } else {
            self.adv_cycles(5);
            self.adv_pc(1);
        }
    }

    // Move Immediate Data
    fn mvi(&mut self, reg: Register) {
        // The MVI instruction uses a 8-bit data quantity, as opposed to
        // LXI which uses a 16-bit data quantity.
        let value = self.read8(self.reg.pc + 1);
        match reg {
            IXH | IXL | IYL | IYH => {
                self.write_reg(reg, value);
                self.adv_cycles(4);
                self.adv_pc(1);
            }
            HL => {
                self.adv_cycles(3);
                let hl = self.read_pair(HL);
                self.memory[hl] = value;
            }
            _ => self.write_reg(reg, value),
        }

        self.adv_cycles(7);
        self.adv_pc(2);
    }

    // LD A, (**)
    fn ld_r_mem_nn(&mut self) {
        let addr = self.read16(self.reg.pc + 1);
        self.reg.a = self.read8(addr);
        self.adv_cycles(13);
        self.adv_pc(3);
    }

    // LD (Load register with value of RP indexed in memory
    fn ld_reg_mem_rp(&mut self, reg: Register) {
        self.reg.a = self.read8(self.read_pair(reg));
        self.adv_cycles(7);
        self.adv_pc(1);
    }

    fn lhld(&mut self, reg: Register) {
        // Load the HL register with 16 bits found at addr & addr + 1
        let imm = self.read16(self.reg.pc + 1);
        self.write_pair(reg, self.read16(imm));
        self.adv_cycles(16);
        self.adv_pc(3);
    }

    pub(crate) fn inc(&mut self, reg: Register) {
        let result = match reg {
            A | B | C | D | E | H | L | HL | IxIm | IyIm | IXH | IXL | IYH | IYL => {
                self.write_reg(reg, self.read_reg(reg).wrapping_add(1));
                self.read_reg(reg)
            }
            _ => panic!("INC on unsupported register"),
        };
        match reg {
            HL => self.adv_cycles(7),
            IxIm | IyIm => {
                self.adv_pc(2);
                self.adv_cycles(19);
            }
            IXH | IXL | IYH | IYL => {
                self.adv_pc(1);
                self.adv_cycles(4);
            }
            _ => {}
        };
        let overflow = (result as i8).wrapping_sub(1).overflowing_add(1).1;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = result == 0;
        self.flags.hf = self.hf_add(result.wrapping_sub(1), 1);
        self.flags.pf = overflow;
        self.flags.nf = false;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.adv_cycles(4);
        self.adv_pc(1);
    }

    fn inc_rp(&mut self, reg: Register) {
        let value = self.read_pair(reg).wrapping_add(1);
        self.write_pair(reg, value);
        if reg == IX || reg == IY {
            self.adv_cycles(4);
            self.adv_pc(1);
        }
        self.adv_cycles(6);
        self.adv_pc(1);
    }

    #[inline]
    fn push(&mut self, reg: Register) {
        self.reg.sp = self.reg.sp.wrapping_sub(2);
        self.write16(self.reg.sp, self.read_pair(reg));
        if reg == IY || reg == IX {
            self.adv_pc(1);
            self.adv_cycles(4);
        }
        self.adv_cycles(11);
        self.adv_pc(1);
    }

    // SBC Subtract Register or Memory from Accumulator with carry flag
    fn sbc(&mut self, dst: Register, src: Register) {
        let value = if src != HL {
            self.read_reg(src) as u16
        } else if src == IyIm || src == IxIm {
            self.adv_pc(2);
            self.adv_cycles(15);
            self.read_reg(src) as u16
        } else {
            self.adv_cycles(3);
            self.memory[self.read_pair(HL)] as u16
        };

        if src == IXL || src == IYL || src == IYH || src == IXH {
            self.adv_cycles(4);
            self.adv_pc(2);
        }

        let result = (dst as u16)
            .wrapping_sub(value)
            .wrapping_sub(self.flags.cf as u16);

        // let overflow = (dst as i8).overflowing_sub((value as i8).overflowing_sub(self.flags.cf as i8).0);
        // self.flags.pf = overflow.1;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = self.hf_sub(self.read_reg(dst), value as u8);
        self.flags.pf = self.overflow(self.read_reg(dst) as i8, value as i8, result as i8);
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.cf = (result & 0x0100) != 0;
        self.flags.nf = true;
        self.write_reg(dst, result as u8);

        self.adv_cycles(4);
        self.adv_pc(1);
    }
    // Extended SBC 0xED42 / 0xED52
    fn sbc_hl(&mut self, reg: Register) {
        let hl = self.read_pair(HL);

        let (result, value): (i32, i32) = (
            (hl as i32)
                .wrapping_sub(self.read_pair(reg) as i32)
                .wrapping_sub(self.flags.cf as i32),
            self.read_pair(reg) as i32,
        );

        self.flags.sf = (result & 0x8000) != 0;
        self.flags.zf = (result & 0xFFFF) == 0;
        self.flags.hf = self.hf_sub_w(hl, value as u16, true);
        self.flags.pf =
            (hl & 0x8000) != (value & 0x8000) as u16 && (hl & 0x8000) != (result & 0x8000) as u16;
        self.flags.yf = (result & 0x2000) != 0;
        self.flags.xf = (result & 0x0800) != 0;
        self.flags.cf = (result & 0x10000) != 0;
        self.flags.nf = true;

        // Write back to HL instead of A unlike normal SBC
        self.write_pair(HL, result as u16);
        self.adv_cycles(15);
        self.adv_pc(2);
    }
    // TODO: SBI & SUI can be consolidated to one function
    // Subtract Immediate with Borrow
    fn sbi(&mut self) {
        let imm = self.read8(self.reg.pc + 1);
        let value = imm + self.flags.cf as u8;
        let result = (self.reg.a as u16).wrapping_sub(value as u16);
        let overflow = (self.reg.a as i8).overflowing_sub(value as i8).1;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = self.hf_sub(self.reg.a, value as u8);
        self.flags.pf = overflow;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.nf = true;
        self.flags.cf = (result & 0x0100) != 0;
        self.reg.a = result as u8;

        self.adv_cycles(7);
        self.adv_pc(2);
    }

    // SUB Subtract Register or Memory From Accumulator
    fn sub(&mut self, src: Register) {
        let value = self.read_reg(src) as u16;
        if src == IXH || src == IYL || src == IXL || src == IYH {
            self.adv_pc(1);
            self.adv_cycles(4);
        };
        if src == HL {
            self.adv_cycles(3);
        }
        if src == IxIm || src == IyIm {
            self.adv_cycles(15);
            self.adv_pc(2);
        }
        let result = (self.reg.a as u16).wrapping_sub(value as u16);
        let overflow = (self.reg.a as i8).overflowing_sub(value as i8).1;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = self.hf_sub(self.reg.a, value as u8);
        self.flags.pf = overflow;
        self.flags.nf = true;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.cf = (result & 0x0100) != 0;
        self.reg.a = result as u8;

        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // SUI Subtract Immediate From Accumulator
    fn sui(&mut self) {
        let value = self.read8(self.reg.pc + 1);
        let result = (self.reg.a as u16).wrapping_sub(value as u16);
        let overflow = (self.reg.a as i8).overflowing_sub(value as i8).1;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = self.hf_sub(self.reg.a, value as u8);
        self.flags.pf = self.overflow(self.reg.a as i8, value as i8, result as i8);
        self.flags.pf = overflow;
        self.flags.nf = true;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.cf = (result & 0x0100) != 0;
        self.reg.a = result as u8;

        self.adv_cycles(7);
        self.adv_pc(2);
    }

    // Set Carry (set carry bit to 1)
    fn scf(&mut self) {
        self.flags.cf = true;
        self.flags.nf = false;
        self.flags.hf = false;
        self.flags.yf = (self.reg.a & 0x20) != 0;
        self.flags.xf = (self.reg.a & 0x08) != 0;
        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // XRA Logical Exclusive-Or memory with Accumulator (Zero accumulator)
    fn xor(&mut self, reg: Register) {
        let value: u16 = if reg != HL {
            self.read_reg(reg) as u16
        } else {
            self.adv_cycles(3);
            self.memory[self.read_pair(HL)] as u16
        };
        if reg == IxIm || reg == IyIm {
            self.adv_pc(2);
            self.adv_pc(15);
        }

        if reg == IXL || reg == IXH || reg == IYL || reg == IYL {
            self.adv_cycles(4);
            self.adv_pc(1);
        }

        let result = self.reg.a as u16 ^ value as u16;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = false;
        self.flags.nf = false;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.cf = false;
        self.flags.pf = self.parity(result as u8);
        self.reg.a = result as u8;

        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // XRI Exclusive-Or Immediate with Accumulator
    fn xri(&mut self) {
        let imm = self.read8(self.reg.pc + 1);
        let result: u8 = self.reg.a ^ imm as u8;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = result == 0;
        self.flags.hf = false;
        self.flags.nf = false;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.pf = self.parity(result);
        self.flags.cf = false;
        self.reg.a = result;

        self.adv_cycles(7);
        self.adv_pc(2);
    }

    fn ex_af_af(&mut self) {
        let a = self.reg.a;
        let a_ = self.reg.a_;
        self.reg.a = a_;
        self.reg.a_ = a;
        self.flags.swap();
        self.adv_cycles(4);
        self.adv_pc(1);
    }
    fn exx(&mut self) {
        let b = self.reg.b;
        let c = self.reg.c;
        let d = self.reg.d;
        let e = self.reg.e;
        let h = self.reg.h;
        let l = self.reg.l;

        self.reg.b = self.reg.b_;
        self.reg.c = self.reg.c_;
        self.reg.d = self.reg.d_;
        self.reg.e = self.reg.e_;
        self.reg.h = self.reg.h_;
        self.reg.l = self.reg.l_;

        self.reg.b_ = b;
        self.reg.c_ = c;
        self.reg.d_ = d;
        self.reg.e_ = e;
        self.reg.h_ = h;
        self.reg.l_ = l;
        self.adv_pc(1);
        self.adv_cycles(4);
    }
    fn ex_de_hl(&mut self) {
        use std::mem;
        mem::swap(&mut self.reg.h, &mut self.reg.d);
        mem::swap(&mut self.reg.l, &mut self.reg.e);
        self.adv_cycles(4);
        self.adv_pc(1);
    }

    fn xthl(&mut self) {
        // Swap H:L with top word on stack
        let hl = self.read_pair(Register::HL) as u16;
        let new_hl = self.read16(self.reg.sp);
        // Write old HL values to memory
        self.write16(self.reg.sp, hl);
        self.write_pair(HL, new_hl);
        self.adv_cycles(19);
        self.adv_pc(1);
    }

    #[inline]
    fn pop(&mut self, reg: Register) {
        self.write_pair(reg, self.read16(self.reg.sp));
        self.reg.sp = self.reg.sp.wrapping_add(2);

        if (reg == IX) || (reg == IY) {
            self.adv_cycles(4);
            self.adv_pc(1);
        }
        self.adv_pc(1);
        self.adv_cycles(10);
    }

    fn ret(&mut self) {
        let low = self.memory[self.reg.sp];
        let high = self.memory[self.reg.sp.wrapping_add(1)];
        let ret: u16 = (high as u16) << 8 | (low as u16);
        // Set program counter for debug output
        self.reg.prev_pc = self.reg.pc;
        self.reg.pc = ret as u16;
        self.reg.sp = self.reg.sp.wrapping_add(2);
        self.adv_cycles(10);
    }

    // Extended opcode
    fn in_c(&mut self, reg: Register) {
        self.write_reg(reg, self.reg.c);
        self.flags.zf = self.read_reg(reg) == 0;
        self.flags.hf = false;
        self.flags.nf = false;
        self.flags.pf = self.parity(self.read_reg(reg));
        self.adv_cycles(12);
        self.adv_pc(2);
    }
    fn in_a(&mut self) {
        self.io.port = self.read8(self.reg.pc + 1);
        self.reg.a = 0xFF; // TODO: hack (other emu's do this for zexdoc??)
                           // self.reg.a = self.io.port;
        self.adv_cycles(11);
        self.adv_pc(2);
    }

    fn out(&mut self, reg: Register) {
        // Set port:
        let port = self.read8(self.reg.pc + 1);
        // println!("Out port: {:02x}, value: {:02x}", port, self.read_reg(reg));
        self.io.value = self.read_reg(reg);
        self.io.port = port;
        self.adv_cycles(11);
        self.adv_pc(2);
    }
    // TODO: Consolidate ORA & ORI (pass value directly)
    fn ora(&mut self, reg: Register) {
        let value = if reg != HL {
            self.read_reg(reg) as u16
        } else {
            self.adv_cycles(3);
            self.memory[self.read_pair(HL)] as u16
        };

        if reg == IxIm || reg == IyIm {
            self.adv_pc(2);
            self.adv_cycles(15);
        }
        let result = self.reg.a as u16 | value as u16;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = false;
        self.flags.nf = false;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.pf = self.parity(result as u8);
        self.flags.cf = false;
        self.reg.a = result as u8;

        self.adv_cycles(4);
        self.adv_pc(1);
    }

    // Or Immediate with Accumulator
    fn ori(&mut self) {
        let result = self.reg.a as u16 | self.read8(self.reg.pc + 1) as u16;

        self.flags.sf = (result & 0x80) != 0;
        self.flags.zf = (result & 0xFF) == 0;
        self.flags.hf = false;
        self.flags.nf = false;
        self.flags.yf = (result & 0x20) != 0;
        self.flags.xf = (result & 0x08) != 0;
        self.flags.pf = self.parity(result as u8);
        self.flags.cf = false;
        self.reg.a = result as u8;

        self.adv_cycles(7);
        self.adv_pc(2);
    }

    // RESET (used for interrupt jump / calls)
    pub fn rst(&mut self, value: u16) {
        // Address to return to after interrupt is finished.
        let ret: u16 = self.reg.pc.wrapping_add(3);
        self.memory[self.reg.sp.wrapping_sub(1)] = (ret >> 8) as u8;
        self.memory[self.reg.sp.wrapping_sub(2)] = ret as u8;
        self.reg.sp = self.reg.sp.wrapping_sub(2);
        self.reg.prev_pc = self.reg.pc;
        self.adv_pc(1);
        self.reg.pc = value;
        self.adv_cycles(11);
    }

    fn sphl(&mut self) {
        self.reg.sp = self.read_pair(HL);
        self.adv_cycles(6);
        self.adv_pc(1);
    }

    // Store H & L direct
    fn shld(&mut self, reg: Register) {
        if reg == IX || reg == IY {
            self.adv_pc(1);
            self.adv_cycles(4);
        }
        let addr = self.read16(self.reg.pc + 1);
        self.write16(addr, self.read_pair(reg));
        self.adv_cycles(16);
        self.adv_pc(3);
    }

    pub fn nop(&mut self) {
        self.adv_pc(1);
        self.adv_cycles(4);
    }

    pub fn execute(&mut self) {
        self.fetch();
        self.decode(self.opcode);
    }

    #[inline]
    pub(crate) fn fetch(&mut self) {
        self.opcode = self.read8(self.reg.pc) as u16;
        self.next_opcode = self.read8(self.reg.pc.wrapping_add(1)) as u16;
    }

    #[inline]
    pub fn decode(&mut self, opcode: u16) {
        use self::Register::*;
        self.reg.r = (self.reg.r & 0x80) | (self.reg.r.wrapping_add(1)) & 0x7f;

        match opcode {
            0x00 => self.nop(),
            0x01 => self.ld_rp_nn(BC),
            0x02 => self.ld(BC, A),
            0x03 => self.inc_rp(BC),
            0x04 => self.inc(B),
            0x05 => self.dec(B),
            0x06 => self.mvi(B),
            0x07 => self.rlca(),
            0x08 => self.ex_af_af(),
            0x09 => self.add_hl(BC),
            0x10 => self.djnz(),

            0x0A => self.ld(A, BC),
            0x0B => self.dec_rp(BC),
            0x0C => self.inc(C),
            0x0D => self.dec(C),
            0x0E => self.mvi(C),
            0x0F => self.rrca(),

            0x11 => self.ld_rp_nn(DE),
            0x12 => self.ld(DE, A),
            0x13 => self.inc_rp(DE),
            0x14 => self.inc(D),
            0x15 => self.dec(D),
            0x16 => self.mvi(D),
            0x17 => self.rla(),
            0x18 => self.jr(self.read8(self.reg.pc) as i16),
            0x19 => self.add_hl(DE),

            0x1A => self.ld_reg_mem_rp(DE),
            0x1B => self.dec_rp(DE),
            0x1C => self.inc(E),
            0x1D => self.dec(E),
            0x1E => self.mvi(E),
            0x1F => self.rra(),

            0x20 => self.jr_cond(!self.flags.zf),
            0x21 => self.ld_rp_nn(HL),
            0x22 => self.shld(HL),
            0x23 => self.inc_rp(HL),
            0x24 => self.inc(H),
            0x25 => self.dec(H),
            0x26 => self.mvi(H),
            0x27 => self.daa(),
            0x28 => self.jr_cond(self.flags.zf),
            0x29 => self.add_hl(HL),

            0x2A => self.lhld(HL),
            0x2B => self.dec_rp(HL),
            0x2C => self.inc(L),
            0x2D => self.dec(L),
            0x2E => self.mvi(L),
            0x2F => self.cpl(),

            0x30 => self.jr_cond(!self.flags.cf),
            0x31 => self.ld_rp_nn(SP),
            0x32 => self.ld_nn_r(),
            0x33 => self.inc_rp(SP),
            0x34 => self.inc(HL),
            0x35 => self.dec(HL),
            0x36 => self.mvi(HL),
            0x37 => self.scf(),
            0x38 => self.jr_cond(self.flags.cf), // JR C, *
            0x39 => self.add_hl(SP),

            0x3A => self.ld_r_mem_nn(),
            0x3B => self.dec_rp(SP),
            0x3C => self.inc(A),
            0x3D => self.dec(A),
            0x3E => self.mvi(A),
            0x3F => self.ccf(),

            // MOV Instructions 0x40 - 0x7F
            0x40 => self.ld(B, B),
            0x41 => self.ld(B, C),
            0x42 => self.ld(B, D),
            0x43 => self.ld(B, E),
            0x44 => self.ld(B, H),
            0x45 => self.ld(B, L),
            0x46 => self.ld(B, HL),
            0x47 => self.ld(B, A),

            0x48 => self.ld(C, B),
            0x49 => self.ld(C, C),
            0x4A => self.ld(C, D),
            0x4B => self.ld(C, E),
            0x4C => self.ld(C, H),
            0x4D => self.ld(C, L),
            0x4E => self.ld(C, HL),
            0x4F => self.ld(C, A),

            0x50 => self.ld(D, B),
            0x51 => self.ld(D, C),
            0x52 => self.ld(D, D),
            0x53 => self.ld(D, E),
            0x54 => self.ld(D, H),
            0x55 => self.ld(D, L),
            0x56 => self.ld(D, HL),
            0x57 => self.ld(D, A),

            0x58 => self.ld(E, B),
            0x59 => self.ld(E, C),
            0x5A => self.ld(E, D),
            0x5B => self.ld(E, E),
            0x5C => self.ld(E, H),
            0x5D => self.ld(E, L),
            0x5E => self.ld(E, HL),
            0x5F => self.ld(E, A),

            0x60 => self.ld(H, B),
            0x61 => self.ld(H, C),
            0x62 => self.ld(H, D),
            0x63 => self.ld(H, E),
            0x64 => self.ld(H, H),
            0x65 => self.ld(H, L),
            0x66 => self.ld(H, HL),
            0x67 => self.ld(H, A),

            0x68 => self.ld(L, B),
            0x69 => self.ld(L, C),
            0x6A => self.ld(L, D),
            0x6B => self.ld(L, E),
            0x6C => self.ld(L, H),
            0x6D => self.ld(L, L),
            0x6E => self.ld(L, HL),
            0x6F => self.ld(L, A),

            0x70 => self.ld(HL, B),
            0x71 => self.ld(HL, C),
            0x72 => self.ld(HL, D),
            0x73 => self.ld(HL, E),
            0x74 => self.ld(HL, H),
            0x75 => self.ld(HL, L),

            0x76 => self.halt(),
            0x77 => self.ld(HL, A),

            0x78 => self.ld(A, B),
            0x79 => self.ld(A, C),
            0x7A => self.ld(A, D),
            0x7B => self.ld(A, E),
            0x7C => self.ld(A, H),
            0x7D => self.ld(A, L),
            0x7E => self.ld_reg_mem_rp(HL),
            0x7F => self.ld(A, A),

            // ADD Instructions
            0x80 => self.add(B),
            0x81 => self.add(C),
            0x82 => self.add(D),
            0x83 => self.add(E),
            0x84 => self.add(H),
            0x85 => self.add(L),
            0x86 => self.add(HL),
            0x87 => self.add(A),

            0x88 => self.adc(B),
            0x89 => self.adc(C),
            0x8A => self.adc(D),
            0x8B => self.adc(E),
            0x8C => self.adc(H),
            0x8D => self.adc(L),
            0x8E => self.adc(HL),
            0x8F => self.adc(A),

            // SUB Instructions
            0x90 => self.sub(B),
            0x91 => self.sub(C),
            0x92 => self.sub(D),
            0x93 => self.sub(E),
            0x94 => self.sub(H),
            0x95 => self.sub(L),
            0x96 => self.sub(HL),
            0x97 => self.sub(A),

            0x98 => self.sbc(A, B),
            0x99 => self.sbc(A, C),
            0x9A => self.sbc(A, D),
            0x9B => self.sbc(A, E),
            0x9C => self.sbc(A, H),
            0x9D => self.sbc(A, L),
            0x9E => self.sbc(A, HL),
            0x9F => self.sbc(A, A),

            // ANA
            0xA0 => self.and(B),
            0xA1 => self.and(C),
            0xA2 => self.and(D),
            0xA3 => self.and(E),
            0xA4 => self.and(H),
            0xA5 => self.and(L),
            0xA6 => self.and(HL),
            0xA7 => self.and(A),

            // XRA
            0xA8 => self.xor(B),
            0xA9 => self.xor(C),
            0xAA => self.xor(D),
            0xAB => self.xor(E),
            0xAC => self.xor(H),
            0xAD => self.xor(L),
            0xAE => self.xor(HL),
            0xAF => self.xor(A),

            // ORA Instructions  0xB(reg)
            0xB0 => self.ora(B),
            0xB1 => self.ora(C),
            0xB2 => self.ora(D),
            0xB3 => self.ora(E),
            0xB4 => self.ora(H),
            0xB5 => self.ora(L),
            0xB6 => self.ora(HL),
            0xB7 => self.ora(A),

            // CMP
            0xB8 => self.cmp(B),
            0xB9 => self.cmp(C),
            0xBA => self.cmp(D),
            0xBB => self.cmp(E),
            0xBC => self.cmp(H),
            0xBD => self.cmp(L),
            0xBE => self.cmp(HL),
            0xBF => self.cmp(A),

            0xC0 => self.ret_cond(!self.flags.zf),
            0xC1 => self.pop(BC),
            0xC2 => self.jp_cond(!self.flags.zf),
            0xC3 => self.jp_cond(true),
            0xC4 => self.call_cond(0xC4, !self.flags.zf),
            0xC5 => self.push(BC),
            0xC6 => self.adi(),
            0xC7 => self.rst(0x0000),
            0xC8 => self.ret_cond(self.flags.zf),
            0xC9 => self.ret(),

            0xCA => self.jp_cond(self.flags.zf),
            0xCB => {
                self.reg.r = (self.reg.r & 0x80) | (self.reg.r.wrapping_add(1)) & 0x7f;
                match self.next_opcode {
                    0x00 => self.rlc(B),
                    0x01 => self.rlc(C),
                    0x02 => self.rlc(D),
                    0x03 => self.rlc(E),
                    0x04 => self.rlc(H),
                    0x05 => self.rlc(L),
                    0x06 => self.rlc(HL),
                    0x08 => self.rrc(B),
                    0x40 => self.bit(0, B),
                    0x41 => self.bit(0, C),
                    0x42 => self.bit(0, D),
                    0x43 => self.bit(0, E),
                    0x44 => self.bit(0, H),
                    0x45 => self.bit(0, L),
                    0x46 => self.bit(0, HL),
                    0x47 => self.bit(0, A),
                    0x48 => self.bit(1, B),
                    0x49 => self.bit(1, C),
                    0x4A => self.bit(1, D),
                    0x4B => self.bit(1, E),
                    0x4C => self.bit(1, H),
                    0x4D => self.bit(1, L),
                    0x4E => self.bit(1, HL),
                    0x4F => self.bit(1, A),

                    0x50 => self.bit(2, B),
                    0x51 => self.bit(2, C),
                    0x52 => self.bit(2, D),
                    0x53 => self.bit(2, E),
                    0x54 => self.bit(2, H),
                    0x55 => self.bit(2, L),
                    0x56 => self.bit(2, HL),
                    0x57 => self.bit(2, A),

                    0x58 => self.bit(3, B),
                    0x59 => self.bit(3, C),
                    0x5A => self.bit(3, D),
                    0x5B => self.bit(3, E),
                    0x5C => self.bit(3, H),
                    0x5D => self.bit(3, L),
                    0x5E => self.bit(3, HL),
                    0x5F => self.bit(3, A),

                    0x60 => self.bit(4, B),
                    0x61 => self.bit(4, C),
                    0x62 => self.bit(4, D),
                    0x63 => self.bit(4, E),
                    0x64 => self.bit(4, H),
                    0x65 => self.bit(4, L),
                    0x66 => self.bit(4, HL),
                    0x67 => self.bit(4, A),

                    0x68 => self.bit(5, B),
                    0x69 => self.bit(5, C),
                    0x6A => self.bit(5, D),
                    0x6B => self.bit(5, E),
                    0x6C => self.bit(5, H),
                    0x6D => self.bit(5, L),
                    0x6E => self.bit(5, HL),
                    0x6F => self.bit(5, A),

                    0x70 => self.bit(6, B),
                    0x71 => self.bit(6, C),
                    0x72 => self.bit(6, D),
                    0x73 => self.bit(6, E),
                    0x74 => self.bit(6, H),
                    0x75 => self.bit(6, L),
                    0x76 => self.bit(6, HL),
                    0x77 => self.bit(6, A),

                    0x78 => self.bit(7, B),
                    0x79 => self.bit(7, C),
                    0x7A => self.bit(7, D),
                    0x7B => self.bit(7, E),
                    0x7C => self.bit(7, H),
                    0x7D => self.bit(7, L),
                    0x7E => self.bit(7, HL),
                    0x7F => self.bit(7, A),

                    0x80 => self.res(0, B),
                    0x81 => self.res(0, C),
                    0x82 => self.res(0, D),
                    0x83 => self.res(0, E),
                    0x84 => self.res(0, H),
                    0x85 => self.res(0, L),
                    0x86 => self.res(0, HL),
                    0x87 => self.res(0, A),

                    0x88 => self.res(1, B),
                    0x89 => self.res(1, C),
                    0x8A => self.res(1, D),
                    0x8B => self.res(1, E),
                    0x8C => self.res(1, H),
                    0x8D => self.res(1, L),
                    0x8E => self.res(1, HL),
                    0x8F => self.res(1, A),

                    0x90 => self.res(2, B),
                    0x91 => self.res(2, C),
                    0x92 => self.res(2, D),
                    0x93 => self.res(2, E),
                    0x94 => self.res(2, H),
                    0x95 => self.res(2, L),
                    0x96 => self.res(2, HL),
                    0x97 => self.res(2, A),

                    0x98 => self.res(3, B),
                    0x99 => self.res(3, C),
                    0x9A => self.res(3, D),
                    0x9B => self.res(3, E),
                    0x9C => self.res(3, H),
                    0x9D => self.res(3, L),
                    0x9E => self.res(3, HL),
                    0x9F => self.res(3, A),

                    0xA0 => self.res(4, B),
                    0xA1 => self.res(4, C),
                    0xA2 => self.res(4, D),
                    0xA3 => self.res(4, E),
                    0xA4 => self.res(4, H),
                    0xA5 => self.res(4, L),
                    0xA6 => self.res(4, HL),
                    0xA7 => self.res(4, A),
                    0xA8 => self.res(5, B),
                    0xA9 => self.res(5, C),
                    0xAA => self.res(5, D),
                    0xAB => self.res(5, E),
                    0xAC => self.res(5, H),
                    0xAD => self.res(5, L),
                    0xAE => self.res(5, HL),
                    0xAF => self.res(5, A),

                    0xB0 => self.res(6, B),
                    0xB1 => self.res(6, C),
                    0xB2 => self.res(6, D),
                    0xB3 => self.res(6, E),
                    0xB4 => self.res(6, H),
                    0xB5 => self.res(6, L),
                    0xB6 => self.res(6, HL),
                    0xB7 => self.res(6, A),
                    0xB8 => self.res(7, B),
                    0xB9 => self.res(7, C),
                    0xBA => self.res(7, D),
                    0xBB => self.res(7, E),
                    0xBC => self.res(7, H),
                    0xBD => self.res(7, L),
                    0xBE => self.res(7, HL),
                    0xBF => self.res(7, A),

                    0xC0 => self.set(0, B),
                    0xC1 => self.set(0, C),
                    0xC2 => self.set(0, D),
                    0xC3 => self.set(0, E),
                    0xC4 => self.set(0, H),
                    0xC5 => self.set(0, L),
                    0xC6 => self.set(0, HL),
                    0xC7 => self.set(0, A),
                    0xC8 => self.set(1, B),
                    0xC9 => self.set(1, C),
                    0xCA => self.set(1, D),
                    0xCB => self.set(1, E),
                    0xCC => self.set(1, H),
                    0xCD => self.set(1, L),
                    0xCE => self.set(1, HL),
                    0xCF => self.set(1, A),

                    0xD0 => self.set(2, B),
                    0xD1 => self.set(2, C),
                    0xD2 => self.set(2, D),
                    0xD3 => self.set(2, E),
                    0xD4 => self.set(2, H),
                    0xD5 => self.set(2, L),
                    0xD6 => self.set(2, HL),
                    0xD7 => self.set(2, A),
                    0xD8 => self.set(3, B),
                    0xD9 => self.set(3, C),
                    0xDA => self.set(3, D),
                    0xDB => self.set(3, E),
                    0xDC => self.set(3, H),
                    0xDD => self.set(3, L),
                    0xDE => self.set(3, HL),
                    0xDF => self.set(3, A),
                    0xE0 => self.set(4, B),
                    0xE1 => self.set(4, C),
                    0xE2 => self.set(4, D),
                    0xE3 => self.set(4, E),
                    0xE4 => self.set(4, H),
                    0xE5 => self.set(4, L),
                    0xE6 => self.set(4, HL),
                    0xE7 => self.set(4, A),
                    0xE8 => self.set(5, B),
                    0xE9 => self.set(5, C),
                    0xEA => self.set(5, D),
                    0xEB => self.set(5, E),
                    0xEC => self.set(5, H),
                    0xED => self.set(5, L),
                    0xEE => self.set(5, HL),
                    0xEF => self.set(5, A),

                    0xF0 => self.set(6, B),
                    0xF1 => self.set(6, C),
                    0xF2 => self.set(6, D),
                    0xF3 => self.set(6, E),
                    0xF4 => self.set(6, H),
                    0xF5 => self.set(6, L),
                    0xF6 => self.set(6, HL),
                    0xF7 => self.set(6, A),
                    0xF8 => self.set(7, B),
                    0xF9 => self.set(7, C),
                    0xFA => self.set(7, D),
                    0xFB => self.set(7, E),
                    0xFC => self.set(7, H),
                    0xFD => self.set(7, L),
                    0xFE => self.set(7, HL),
                    0xFF => self.set(7, A),
                    _ => unimplemented!(
                        "Unknown 0xCB opcode:{:02X}{:02X}",
                        self.opcode,
                        self.next_opcode
                    ),
                }
            }
            0xCC => self.call_cond(0xCC, self.flags.zf),
            0xCD => self.call(0xCD),
            0xCE => self.adc_im(),
            0xCF => self.rst(0x0008),

            0xD0 => self.ret_cond(!self.flags.cf),
            0xD1 => self.pop(DE),
            0xD2 => self.jp_cond(!self.flags.cf),
            0xD3 => self.out(A),
            0xD4 => self.call_cond(0xD4, !self.flags.cf),
            0xD5 => self.push(DE),
            0xD6 => self.sui(),
            0xD7 => self.rst(0x0010),
            0xD8 => self.ret_cond(self.flags.cf),
            0xD9 => self.exx(),
            0xDA => self.jp_cond(self.flags.cf),
            0xDB => self.in_a(),
            0xDC => self.call_cond(0xDC, self.flags.cf),
            0xDD => {
                self.reg.r = (self.reg.r & 0x80) | self.reg.r.wrapping_add(1) & 0x7f;
                match self.read8(self.reg.pc + 1) {
                    0x09 => self.add_rp(IX, BC),
                    0x19 => self.add_rp(IX, DE),
                    0x21 => self.ld_rp_nn(IX),
                    0x22 => self.shld(IX),
                    0x23 => self.inc_rp(IX),
                    0x24 => self.inc(IXH),
                    0x25 => self.dec(IXH),
                    0x26 => self.mvi(IXH),
                    0x29 => self.add_rp(IX, IX),
                    0x2A => {
                        self.lhld(IX);
                        self.adv_cycles(4);
                        self.adv_pc(1);
                    }
                    0x2B => self.dec_rp(IX),
                    0x2C => self.inc(IXL),
                    0x2D => self.dec(IXL),
                    0x2E => self.mvi(IXL),
                    0x34 => self.inc(IxIm),
                    0x35 => self.dec(IxIm),
                    0x36 => {
                        self.write8(
                            self.reg.ix + self.read8(self.reg.pc + 1) as u16,
                            self.read8(self.reg.pc + 1),
                        );
                        self.adv_cycles(19);
                        self.adv_pc(4);
                    }
                    0x39 => self.add_rp(IX, SP),
                    0x3C => unimplemented!("{:04x}", self.next_opcode),
                    0x3D => unimplemented!("{:04x}", self.next_opcode),
                    0x3E => unimplemented!("{:04x}", self.next_opcode),
                    0x44 => self.ld(B, IXH),
                    0x45 => self.ld(B, IXL),
                    0x46 => self.ld(B, IxIm),
                    0x4C => self.ld(C, IXH),
                    0x4D => self.ld(C, IXL),
                    0x4E => self.ld(C, IxIm),
                    0x54 => self.ld(D, IXH),
                    0x55 => self.ld(D, IXL),
                    0x56 => self.ld(D, IxIm),
                    0x5C => self.ld(E, IXH),
                    0x5D => self.ld(E, IXL),
                    0x5E => self.ld(E, IxIm),
                    0xE1 => self.pop(IX),
                    0xE5 => self.push(IX),
                    0x60 => self.ld(IXH, B),
                    0x61 => self.ld(IXH, C),
                    0x62 => self.ld(IXH, D),
                    0x63 => self.ld(IXH, E),
                    0x64 => self.ld(IXH, IXH),
                    0x65 => self.ld(IXH, IXL),
                    0x66 => self.ld(H, IxIm),
                    0x67 => self.ld(IXH, A),
                    0x68 => self.ld(IXL, B),
                    0x69 => self.ld(IXL, C),
                    0x6A => self.ld(IXL, D),
                    0x6B => self.ld(IXL, E),
                    0x6C => self.ld(IXL, IXH),
                    0x6D => self.ld(IXL, IXL),
                    0x6E => self.ld(L, IxIm),
                    0x6F => self.ld(IXL, A),
                    0x7E => {
                        // byte is the signed displacement byte
                        let byte = self.read8(self.reg.pc + 2) as i8;
                        let addr = self.reg.ix.wrapping_add(byte as u16);
                        self.reg.a = self.read8(addr) as i8 as u8;
                        self.adv_pc(3);
                        self.adv_cycles(19);
                    }
                    0x77 => self.ld(A, IxIm),
                    0x84 => self.add(IXH),
                    0x85 => self.add(IXL),
                    0x86 => self.add(IxIm),
                    0x8C => self.adc(IXH),
                    0x8D => self.adc(IXL),
                    0x8E => self.adc(IxIm),
                    0x94 => self.sub(IXH),
                    0x95 => self.sub(IXL),
                    0x9C => self.sbc(A, IXH),
                    0x9D => self.sbc(A, IXL),
                    0x9E => self.sbc(A, IxIm),
                    0x96 => self.sub(IxIm),
                    0xA4 => self.and(IXH),
                    0xA5 => self.and(IXL),
                    0xA6 => self.add(IxIm),
                    0xAC => self.xor(IXH),
                    0xAD => self.xor(IXL),
                    0xAE => self.xor(IxIm),
                    0xB4 => self.ora(IXH),
                    0xB5 => self.ora(IXL),
                    0xB6 => self.ora(IxIm),
                    0xBC => self.cmp(IXH),
                    0xBD => self.cmp(IXH),
                    0xBE => self.cmp(IxIm),
                    // DDCB
                    0xCB => {
                        // self.next_opcode = self.read8(self.reg.pc.wrapping_add(1)) as u16;
                        match self.read8(self.reg.pc + 2) {
                            0x00 => self.rlc(B),
                            0x01 => self.rlc(C),
                            0x02 => self.rlc(D),
                            0x03 => self.rlc(E),
                            0x04 => self.rlc(H),
                            0x05 => self.rlc(L),
                            0x06 => self.rlc(HL),
                            _ => unimplemented!(
                                "DDCB instruction: Opcode:{:02X}{:02X}{:02X}",
                                self.opcode,
                                self.next_opcode,
                                self.read8(self.reg.pc + 2)
                            ),
                        }
                    }
                    0xE9 => self.jp(self.reg.ix, 8),

                    _ => {
                        eprintln!("Last instruction:{:#?}", Instruction::decode(self));
                        unimplemented!(
                            "Unimplemented DD instruction: {:02X}{:02X}{:02X}",
                            self.opcode,
                            self.next_opcode,
                            self.read8(self.reg.pc + 2)
                        )
                    }
                }
            }
            0xDE => self.sbi(),
            0xDF => self.rst(0x0018),
            0xE0 => self.ret_cond(!self.flags.pf),
            0xE1 => self.pop(HL),
            0xE2 => self.jp_cond(!self.flags.pf),
            0xE3 => self.xthl(),
            0xE4 => self.call_cond(0xE4, !self.flags.pf),
            0xE5 => self.push(HL),
            0xE6 => self.ani(),
            0xE7 => self.rst(0x0020),
            0xE8 => self.ret_cond(self.flags.pf),
            0xE9 => self.pchl(),

            0xEA => self.jp_cond(self.flags.pf),
            0xEB => self.ex_de_hl(),
            0xEC => self.call_cond(0xEC, self.flags.pf),
            0xED => {
                self.reg.r = (self.reg.r & 0x80) | (self.reg.r.wrapping_add(1)) & 0x7f;
                match self.next_opcode {
                    0x08 => self.in_c(C),
                    0xA0 => self.ldi(),
                    0xA1 => self.cpi(),
                    0xA9 => self.cpd(),
                    0xB0 => self.ldir(),
                    0x42 => self.sbc_hl(BC),
                    0x43 => self.ld_mem_nn_rp(BC),
                    0x46 => self.set_interrupt_mode(0),
                    0x47 => self.ld(I, A),
                    0x4A => self.adc_hl(BC),
                    0x4B => self.ld_rp_mem_nn(BC),
                    0x4F => self.ld(R, A),
                    0x50 => self.in_c(D),
                    0x52 => self.sbc_hl(DE),
                    0x53 => self.ld_mem_nn_rp(DE),
                    0x5E => self.set_interrupt_mode(2),
                    0x56 => self.set_interrupt_mode(1),
                    0x57 => self.ld(A, I),
                    0x5F => self.ld(A, R),
                    0x5A => self.adc_hl(DE),
                    0x5B => self.ld_rp_mem_nn(DE),
                    0x62 => self.sbc_hl(HL),
                    0x63 => self.ld_mem_nn_rp(HL),
                    0x66 => self.set_interrupt_mode(0),
                    0x6A => self.adc_hl(HL),
                    0x6B => self.ld_rp_mem_nn(HL),
                    0x72 => self.sbc_hl(SP),
                    0x73 => self.ld_mem_nn_rp(SP),
                    0x76 => self.set_interrupt_mode(1),
                    0x7B => self.ld_rp_mem_nn(SP),
                    0x7A => self.adc_hl(SP),
                    0x7E => self.set_interrupt_mode(2),
                    0xB1 => self.cpir(),
                    0xB9 => self.cpdr(),
                    _ => unimplemented!(
                        "Unimplemented ED instruction:{:02X}{:02X}",
                        self.opcode,
                        self.next_opcode
                    ),
                }
            }

            0xEE => self.xri(),
            0xEF => self.rst(0x0028),
            0xF0 => self.ret_cond(!self.flags.sf),
            0xF1 => self.pop(AF),
            0xF2 => self.jp_cond(!self.flags.sf),
            0xF3 => self.interrupt(false),
            0xF4 => self.call_cond(0xF4, !self.flags.sf),
            0xF5 => self.push(AF),
            0xF6 => self.ori(),
            0xF7 => self.rst(0x0020),
            0xF8 => self.ret_cond(self.flags.sf),
            0xF9 => self.sphl(),
            0xFA => self.jp_cond(self.flags.sf),
            0xFB => self.interrupt(true),
            0xFC => self.call_cond(0xFC, self.flags.sf),
            0xFD => {
                self.reg.r = (self.reg.r & 0x80) | (self.reg.r.wrapping_add(1)) & 0x7f;
                match self.next_opcode {
                    0x09 => self.add_rp(IY, BC),
                    0x6E => self.ld(L, IyIm),
                    0x6F => self.ld(IYL, A),
                    0x19 => self.add_rp(IY, DE),
                    0x21 => self.ld_rp_nn(IY),
                    0x22 => self.shld(IY),
                    0x23 => self.inc_rp(IY),
                    0x26 => {
                        // self.ld(IYH, *)
                        self.write_reg(IYH, self.read8(self.reg.pc + 1));
                        self.adv_pc(3);
                        self.adv_cycles(11);
                    }
                    0x29 => self.add_rp(IY, IY),
                    0x2A => {
                        self.lhld(IY);
                        self.adv_cycles(4);
                        self.adv_pc(1);
                    }
                    0x2B => self.dec_rp(IY),
                    0x2E => self.mvi(IYL),
                    0x24 => self.inc(IYH),
                    0x25 => self.dec(IYH),
                    0x2C => self.inc(IYH),
                    0x2D => self.inc(IYL),
                    0x34 => self.inc(IyIm),
                    0x35 => self.dec(IyIm),
                    0x36 => {
                        self.write8(
                            self.reg.iy + self.read8(self.reg.pc + 1) as u16,
                            self.read8(self.reg.pc + 1),
                        );
                        self.adv_cycles(19);
                        self.adv_pc(4);
                    }
                    0x39 => self.add_rp(IY, SP),
                    0x44 => self.ld(B, IYH),
                    0x45 => self.ld(B, IYL),
                    0x46 => self.ld(B, IyIm),
                    0x4C => self.ld(C, IYH),
                    0x4D => self.ld(C, IYL),
                    0x4E => self.ld(C, IyIm),
                    0x54 => self.ld(D, IYH),
                    0x55 => self.ld(D, IYL),
                    0x56 => self.ld(D, IyIm),
                    0x5C => self.ld(E, IYH),
                    0x5D => self.ld(E, IYL),
                    0x5E => self.ld(E, IyIm),

                    0xE1 => self.pop(IY),
                    0xE5 => self.push(IY),
                    0xE9 => self.jp(self.read_pair(IY), 8),
                    0x66 => {
                        let byte = self.read8(self.reg.pc + 2);
                        let addr = self.reg.iy.wrapping_add(byte as u16);
                        self.reg.h = self.read8(addr) as u8;
                        self.adv_pc(3);
                        self.adv_cycles(19);
                    }
                    0x7E => {
                        // byte is the signed displacement byte
                        let byte = self.read8(self.reg.pc + 2) as i8;
                        let addr = self.reg.iy.wrapping_add(byte as u16);
                        self.reg.a = self.read8(addr) as i8 as u8;
                        self.adv_pc(3);
                        self.adv_cycles(19);
                    }

                    0x84 => self.add(IYH),
                    0x85 => self.add(IYL),
                    0x86 => self.add(IyIm),
                    0x8C => self.adc(IYH),
                    0x8D => self.adc(IYL),
                    0x8E => self.adc(IyIm),

                    0x94 => self.sub(IYH),
                    0x95 => self.sub(IYL),
                    0x96 => self.sub(IyIm),
                    0x9C => self.sbc(A, IYH),
                    0x9D => self.sbc(A, IYL),
                    0x9E => self.sbc(A, IyIm),
                    0xA4 => self.and(IYH),
                    0xA5 => self.and(IYL),
                    0xA6 => self.and(IyIm),
                    0xAC => self.xor(IYH),
                    0xAD => self.xor(IYL),
                    0xAE => self.xor(IyIm),
                    0xB4 => self.ora(IYH),
                    0xB5 => self.ora(IYL),
                    0xB6 => self.ora(IyIm),
                    0xBC => self.cmp(IYH),
                    0xBD => self.cmp(IYH),
                    0xBE => self.cmp(IxIm),
                    0xCB => {
                        let next_opcode = self.read8(self.reg.pc + 2);
                        match next_opcode {
                            0x00 => self.rlc_ex(IyIm, B),
                            0x01 => self.rlc_ex(IyIm, C),
                            0x02 => self.rlc_ex(IyIm, D),
                            0x03 => self.rlc_ex(IyIm, E),
                            0x04 => self.rlc_ex(IyIm, H),
                            0x05 => self.rlc_ex(IyIm, L),
                            _ => unimplemented!("DDCB:{:02X}", next_opcode),
                        }
                    }
                    _ => panic!("{:#?}", Instruction::decode(self)),
                }
            }
            0xFE => self.cp_im(),
            0xFF => self.rst(0x0038),
            _ => panic!(
                "Unknown or unimplemented instruction:{:#?}",
                Instruction::decode(self)
            ),
        }
    }

    pub fn reset(&mut self) {
        self.reg.a = 0xff;
        self.reg.b = 0;
        self.reg.c = 0;
        self.reg.d = 0;
        self.reg.e = 0;
        self.reg.h = 0;
        self.reg.l = 0;
        self.reg.sp = 0xffff;
        self.reg.r = 0;
        // Reset flag conditions
        self.flags.set(0xff);
        self.int.mode = 0;
        self.int.iff1 = false;
        self.int.iff2 = false;
        self.int.halt = false;
    }

    // http://www.z80.info/z80syntx.htm#HALT
    fn halt(&mut self) {
        self.int.halt = true;
        // self.int.nmi_pending = true; // We're pending on an interrupt, finish this instruction first
        self.adv_cycles(4);
        self.nop();
    }

    fn parity(&self, value: u8) -> bool {
        // Check parity against LSB only
        value.count_ones() & 1 == 0
    }

    fn hf_add(&self, a: u8, b: u8) -> bool {
        // ((((a & 0xF) + (b & 0xF)) & 0x10) & (1 << 4)) != 0
        (((a as i8 & 0x0F).wrapping_add(b as i8 & 0x0F)) & 0x10) != 0
    }

    fn hf_add_w(&self, a: u16, b: u16, carry: bool) -> bool {
        // ((a & 0x0FFF) + (b & 0x0FFF)) & 0x1000 & (1 << 12) != 0
        if !carry {
            (((a & 0x0FFF).wrapping_add(b & 0x0FFF)) & (1 << 12)) != 0
        } else {
            (((a & 0xFFF) + (b & 0xFFF) + self.flags.cf as u16) & (1 << 12)) != 0
        }
    }

    fn hf_sub(&self, a: u8, b: u8) -> bool {
        // Check if there has been a borrow from bit 4
        (((a as i8 & 0xF) - (b as i8 & 0xF)) & (1 << 4)) != 0
    }
    fn hf_sub_w(&self, a: u16, b: u16, carry: bool) -> bool {
        // True if there has been a borrow from bit 12
        // In the case for ADC or SBC instructions we need to take the carry flag into account
        if !carry {
            (((a & 0xFFF) - (b & 0xFFF)) & (1 << 12)) != 0
        } else {
            ((a & 0xFFF).wrapping_sub((b & 0xFFF) as u16 + self.flags.cf as u16) & (1 << 12)) != 0
        }
    }

    fn overflow(&mut self, a: i8, b: i8, result: i8) -> bool {
        // Overflow should be set if the 2-complement result does not fit the register
        // Set overflow flag when A and the B have the same sign
        // and A and the result have different sign
        (a.wrapping_shr(7) == (b.wrapping_shr(7)))
            && ((a.wrapping_shr(7)) != (result.wrapping_shr(7)))
    }

    pub(crate) fn poll_interrupt(&mut self) {
        // Accepting an NMI
        if self.int.nmi_pending {
            self.int.nmi_pending = false;
            self.int.iff1 = false;
            self.int.halt = false;
            self.reg.r = self.reg.r.wrapping_add(1);
            self.adv_cycles(11);
            self.rst(0x66);
            return;
        }
        if (self.int.nmi_pending || self.int.irq) || self.int.iff1 {
            self.int_pending = false;
            self.int.halt = false;
            self.int.iff1 = false;
            self.int.iff2 = false;
            self.reg.r = (self.reg.r & 0x80) | (self.reg.r.wrapping_add(0) as u8 & 0x7f);

            // Interrupt Mode 0 is the 8080 compatibility mode
            // Most commonly the instruction executed on the bus is RST,
            // but it can be any instruction (technically)
            // The I register is not used for IM0
            // TODO investigate interrupt processing
            match self.int.mode {
                0 => {
                    if self.int.vector != 0 || self.io.input {
                        self.adv_cycles(11);
                        if self.debug {
                            println!("Servicing interrupt, mode 0");
                        }
                        self.decode(self.int.vector as u16);
                    }
                }
                1 => {
                    // Mode 1, RST38h, regardless of bus value or I reg value.
                    if self.debug {
                        println!("Servicing interrupt, mode 1");
                    }
                    self.adv_cycles(13);
                    self.rst(0x38);
                }
                2 => {
                    // http://z80.info/1653.htm Interrupt MODE 2 details
                    self.adv_cycles(2);
                    if self.io.port == 0 {
                        self.int.vector = self.io.value;
                    }
                    // The interrupt vector is two part, composed by the I register and the lower
                    // 8-bits of the vector is placed on the bus. The resulting address is a vector
                    // that points to the beginning of RAM, the resulting address from reading this
                    // is the interrupt handler routine.
                    // let vector = self.read16((self.reg.i.wrapping_shl(8) | self.int.vector) as u16);
                    let vector = self.reg.i.wrapping_shl(8) | self.io.value;
                    self.call(vector as u16);

                    self.int.int = false;
                    self.int.irq = false;
                    if self.debug {
                        println!("Servicing interrupt: Mode 2");
                    }
                }
                _ => panic!("Unhandled interrupt mode"),
            }
        }
    }
}
