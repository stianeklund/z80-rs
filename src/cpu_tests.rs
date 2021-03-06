#[cfg(test)]
mod tests {
    use crate::instruction_info::Register;
    use crate::instruction_info::Register::{BC, DE, HL, IX, IXH, IY, R, SP};
    use crate::interconnect::Interconnect;
    use crate::memory::MemoryRW;

    #[test]
    fn test_overflow_flag_add() {
        let mut i = Interconnect::default();
        i.cpu.reg.a = 0b0110_0100;
        i.cpu.reg.b = 0b0011_0001;
        i.cpu.add(Register::B);
        assert_eq!(i.cpu.flags.pf, true);
    }
    #[test]
    fn test_overflow_flag_sub() {
        let mut i = Interconnect::default();
        i.cpu.reg.a = 0b0111_1110;
        i.cpu.reg.b = 0b1100_0000;
        i.cpu.sub(Register::B);
        assert_eq!(i.cpu.flags.pf, true);
    }

    #[test]
    #[ignore]
    fn test_ld_hl_indexed() {
        // Ignore for now; don't actually remember if this ever passed if it did it's now failing
        // and we have a regression; however compared to previous commit: 596d4ce
        // we have no known new regressions with zexdoc either!
        let mut i = Interconnect::default();
        i.cpu.write8(0x1E07, 0x77);
        i.cpu.reg.a = 0xff;
        i.cpu.write_pair(HL, 0x1E07);
        i.cpu.ld(HL, Register::A);
        assert_eq!(i.cpu.read8(0x1E07), 0xff);
    }

    #[test]
    fn test_hf_flag() {
        // Make sure HF flag gets set on accumulator value wrap from FFh to 00h.
        let mut i = Interconnect::default();
        i.cpu.reg.a = 0xff;
        i.cpu.inc(Register::A);
        assert_eq!(i.cpu.flags.hf, true);
    }

    #[test]
    fn test_ld_ixh_ixh() {
        let mut i = Interconnect::default();
        i.cpu.reg.a = 0xff;
        i.cpu.reg.ix = 0xfff0;
        i.cpu.ld(Register::IXH, Register::IXH);
        assert_eq!(i.cpu.reg.ix, 0xfff0);
        assert_eq!(i.cpu.cycles, 8);
        assert_eq!(i.cpu.reg.pc, 2);
    }

    #[test]
    fn test_hf_high_byte() {
        // The half carry flag should be set once we increment HL from 00FFh to 0000h
        let mut i = Interconnect::default();
        i.cpu.write_pair(BC, 1); // Set BC to 1 (we will increment HL by 1)
        i.cpu.reg.a = 0xff;
        i.cpu.write_pair(HL, 0x00FF);
        i.cpu.add_hl(BC);
        i.cpu.inc(Register::A);
        assert_eq!(i.cpu.flags.hf, true);
    }

    #[test]
    fn test_add_half_carry() {
        // Replicates a scenario in Zexdoc where HF flag was not set
        // due to the half carry not being tested with `a + b + carry` but only `a + b`
        // TODO: Write separate test to cover HF flag more generally for both ADC and SBC
        let mut i = Interconnect::default();
        i.cpu.reg.pc = 0x1CBE;
        i.cpu.reg.a = 0x6F;
        i.cpu.flags.set(0x11);
        i.cpu.write_pair(BC, 0x0B29);
        i.cpu.write_pair(BC, 0x5B61);
        i.cpu.write_pair(HL, 0xDF6D);
        i.cpu.write_pair(SP, 0x85B2);
        i.cpu.write_pair(IX, 0x7A67);
        i.cpu.write_pair(IY, 0x7E3C);
        i.cpu.write_reg(R, 0x09);
        i.cpu.cycles = 307892903;
        // Expected values: value = 01; carry = 0; result = 68;
        i.cpu.adc_im();
        assert_eq!(i.cpu.flags.hf, true);
    }

    #[test]
    fn fast_z80() {
        // Assert the tests executed CPU cycle amount vs real hardware cycle
        assert_eq!(exec_test("tests/prelim.com"), 8721);
        assert_eq!(exec_test("tests/8080PRE.COM"), 7772);
        assert_eq!(exec_test("tests/CPUTEST.COM"), 240551424);
    }

    #[test]
    #[ignore] // Ignored for now as they do not pass
    // zexdoc.cim is a custom binary compiled with zmac where certain tests are stubbed
    fn z80_precise() {
        assert_eq!(exec_test("tests/zexdoc.com"), 46734978649);
        // assert_eq!(exec_test("tests/zexdoc.cim"), 46734978649);
        // assert_eq!(exec_test("tests/zexall.com"), 46734978649);
    }

    // #[test]
    fn all_tests() {
        assert_eq!(exec_test("tests/prelim.com"), 8721);
        assert_eq!(exec_test("tests/8080PRE.COM"), 7772);
        assert_eq!(exec_test("tests/CPUTEST.COM"), 240551424);
        assert_eq!(exec_test("tests/zexall.com"), 46734978649);
        assert_eq!(exec_test("tests/zexdoc.com"), 46734978649);
    }

    fn exec_test(bin: &str) -> usize {
        let mut i = Interconnect::default();
        i.cpu.reset();
        i.cpu.memory.load_tests(bin);

        // Patches the test rom(s) to intercept CP/M bdos routine
        // Inject OUT *, A at 0x0000.
        // Inject RET (0xC9) at 0x0007 to handle the return call.
        // Inject IN, A * to store BDOS output
        // If successful it should return to 0x0007.

        i.cpu.memory.rom[0x0000] = 0xD3;
        i.cpu.memory.rom[0x0001] = 0x00;
        i.cpu.memory.rom[0x0005] = 0xDB;
        i.cpu.memory.rom[0x0006] = 0x00;
        i.cpu.memory.rom[0x0007] = 0xC9;

        // All test binaries start at 0x0100.
        i.cpu.reg.pc = 0x0100;

        // Turn CPM Compatibility on. This turns off any memory mapping
        i.cpu.cpm_compat = true;
        // i.cpu.debug = true;

        loop {
            //if i.cpu.cycles >= 126729335 {
            //    i.cpu.debug = true;
            //}

            i.run_tests();
            if i.cpu.reg.pc == 0x76 {
                assert_ne!(i.cpu.reg.pc, 0x76);
            }

            if i.cpu.reg.pc == 07 {
                if i.cpu.reg.c == 9 {
                    let mut de = i.cpu.read_pair(DE);
                    'print: loop {
                        let output = i.cpu.memory.rom[de as usize];
                        if output as char == '$' {
                            break 'print;
                        } else if output as char != '$' {
                            de += 1;
                        }
                        print!("{}", output as char);
                    }
                }
                if i.cpu.reg.c == 2 {
                    print!("{}", i.cpu.reg.e as char);
                }
            }
            if i.cpu.opcode == 0xD3 {
                break;
            } else if i.cpu.reg.pc == 0 {
                {
                    println!(
                        "\nBDOS routine called, jumped to: 0 from {:04X}",
                        i.cpu.reg.prev_pc
                    );
                }
            }
        }
        println!("Cycles executed: {}\n", i.cpu.cycles);

        i.cpu.cycles
    }
}
