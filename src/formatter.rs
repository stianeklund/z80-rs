use crate::cpu::{Cpu, Registers};
use crate::memory::MemoryRW;
use std::fmt;
use std::fmt::{Debug, Display, Formatter, Result};

impl Display for Registers {
    fn fmt(&self, fmt: &mut Formatter) -> Result {
        fmt.debug_struct("Registers")
            .field("PC", &format_args!("{:04x}", self.prev_pc))
            .field("A", &format_args!("{:02x}", self.a))
            .field("BC", &format_args!("{:02x},{:02x}", self.b, self.c))
            .field("DE", &format_args!("{:02x},{:02x}", self.d, self.e))
            .field("HL", &format_args!("{:02x},{:02x}", self.h, self.l))
            .field("SP", &format_args!("{:04x}", self.sp))
            .finish()
    }
}

impl Debug for Cpu {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        fmt.align();
        write!(fmt, "PC: {:>04X}, ", self.reg.pc)?;
        write!(fmt, "AF: {:>02X}{:02X}, ", self.reg.a, self.flags.get())?;
        write!(fmt, "BC: {:>02X}{:02X}, ", self.reg.b, self.reg.c)?;
        write!(fmt, "DE: {:>02X}{:02X}, ", self.reg.d, self.reg.e)?;
        write!(fmt, "HL: {:>02X}{:02X}, ", self.reg.h, self.reg.l)?;
        write!(fmt, "SP: {:>04X}, ", self.reg.sp)?;
        write!(fmt, "IX: {:>04X}, ", self.reg.ix)?;
        write!(fmt, "IY: {:>04X}, ", self.reg.iy)?;
        write!(fmt, "I: {:02X}, ", self.int.int as u8)?;
        write!(fmt, "R: {:02X}\t", self.reg.r as u8)?;
        write!(
            fmt,
            "({:02X} {:02X} {:02X} {:02X}), ",
            self.read8(self.reg.pc),
            self.read8(self.reg.pc.wrapping_add(1)),
            self.read8(self.reg.pc.wrapping_add(2)),
            self.read8(self.reg.pc.wrapping_add(3))
        )?;
        write!(fmt, "cyc: {}", self.cycles)
    }
}
impl Display for Cpu {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        fmt.align();
        write!(fmt, "{:w$}", &self.current_instruction, w = 12)?;
        write!(
            fmt,
            "({:02X} {:02X} {:02X} {:02X})\t",
            self.read8(self.reg.pc),
            self.read8(self.reg.pc.wrapping_add(1)),
            self.read8(self.reg.pc.wrapping_add(2)),
            self.read8(self.reg.pc.wrapping_add(3))
        )?;
        write!(fmt, "Opcode: ")?;
        write!(fmt, "{:>04X}\t", self.opcode)?;
        write!(fmt, "PC:{:>04X}\t", self.reg.pc)?;
        write!(fmt, "AF:{:>02X}{:02X}\t", self.reg.a, self.flags.get())?;
        write!(fmt, "BC:{:>02X}{:02X}\t", self.reg.b, self.reg.c)?;
        write!(fmt, "DE:{:>02X}{:02X}\t", self.reg.d, self.reg.e)?;
        write!(fmt, "HL:{:>02X}{:02X}\t", self.reg.h, self.reg.l)?;
        write!(fmt, "IX:{:>04X}\t", self.reg.ix)?;
        write!(fmt, "IY:{:>04X}\t", self.reg.iy)?;
        write!(fmt, "SP:{:>04X}\t", self.reg.sp)?;
        write!(fmt, "S:{} ", self.flags.sf as u8)?;
        write!(fmt, "Z:{} ", self.flags.zf as u8)?;
        write!(fmt, "P:{} ", self.flags.pf as u8)?;
        write!(fmt, "C:{} ", self.flags.cf as u8)?;
        write!(fmt, "H:{} ", self.flags.hf as u8)?;
        write!(fmt, "I:{}", self.int.int as u8)
    }
}

/*// TODO Refactor the above to fit this style
impl Debug for Cpu {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            fmt,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}    \t{}\t{}\t{}\t{}\t{}\t{}\t",
            "Instruction",
            "Opcode",
            "PC",
            "A",
            "BC",
            "DE",
            "HL",
            "SP",
            "S   ",
            "Z   ",
            "P   ",
            "C   ",
            "AC   ",
            "I   "
        )?;
        writeln!(
            fmt,
            "{}\t{:04X}\t{:04X}\t{:02X}\t{:02X}{:02X}\t{:02X}{:02X}\t{:02X}{:02X}\t{:0>4X}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.current_instruction,
            self.opcode,
            self.reg.prev_pc,
            self.reg.a,
            self.reg.b,
            self.reg.c,
            self.reg.d,
            self.reg.e,
            self.reg.h,
            self.reg.l,
            self.reg.sp,
            self.flags.sf as u8,
            self.flags.zf as u8,
            self.flags.pf as u8,
            self.flags.cf as u8,
            self.flags.hf as u8,
            self.irq.int as u8
        )
    }
}*/
