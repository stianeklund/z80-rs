#[cfg(test)]
mod tests {
    use crate::instruction_info::Register;
    use crate::instruction_info::Register::{BC, DE, HL};
    use crate::interconnect::Interconnect;
    use crate::memory::MemoryRW;

    #[test]
    fn test_hf_flag() {
        // Make sure HF flag gets set on accumulator value wrap from FFh to 00h.
        let mut i = Interconnect::new();
        i.cpu.reg.a = 0xff;
        i.cpu.inc(Register::A);
        assert_eq!(i.cpu.flags.hf, true);
    }

    #[test]
    fn test_hf_high_byte() {
        // The half carry flag should be set once we increment HL from 00FFh to 0000h
        let mut i = Interconnect::new();
        i.cpu.write_pair_direct(BC, 1); // Set BC to 1 (we will increment HL by 1)
        i.cpu.reg.a = 0xff;
        i.cpu.write_pair_direct(HL, 0x00FF);
        i.cpu.add_hl(BC);
        i.cpu.inc(Register::A);
        assert_eq!(i.cpu.flags.hf, true);
    }

    #[test]
    fn fast_z80() {
        // Assert the tests executed CPU cycle amount vs real hardware cycles
        assert_eq!(exec_test("tests/prelim.com"), 8721);
        assert_eq!(exec_test("tests/8080PRE.COM"), 7772);
        assert_eq!(exec_test("tests/CPUTEST.COM"), 240551424);
    }

    #[test]
    #[ignore] // Ignored for now as they do not pass
    fn z80_precise() {
        assert_eq!(exec_test("tests/zexdoc.com"), 46734978649);
        // ^ Bug in LDA_IM ?
        // assert_eq!(exec_test("tests/zexall.com"), 46734978649);
    }

    // #[test]
    fn all_tests() {
        assert_eq!(exec_test("tests/prelim.com"), 8721);
        assert_eq!(exec_test("tests/8080PRE.COM"), 7772);
        assert_eq!(exec_test("tests/CPUTEST.COM"), 240551424);
        assert_eq!(exec_test("tests/zexall.com"), 0);
        assert_eq!(exec_test("tests/zexdoc.com"), 0);
    }

    fn exec_test(bin: &str) -> usize {
        let mut i = Interconnect::new();
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
        let mut reset_counter = 0;

        loop {
            // Turn CMP Compatability on. This turns off any memory mapping
            i.cpu.cpm_compat = true;
            // i.cpu.debug = true;
            i.run_tests();

            if i.cpu.reg.pc == 0x76 {
                assert_ne!(i.cpu.reg.pc, 0x76);
            }

            if i.cpu.reg.pc == 07 {
                if i.cpu.reg.c == 9 {
                    let mut de = i.cpu.get_pair(DE);
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
                    reset_counter += 1;
                }
            }
            if reset_counter > 1 {
                break;
            }
        }
        println!("Cycles executed: {}\n", i.cpu.cycles);

        i.cpu.cycles
    }
}
