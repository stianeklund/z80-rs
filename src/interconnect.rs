use super::cpu::Cpu;
use crate::instruction_info::Instruction;

pub struct Interconnect {
    pub cpu: Cpu,
    pub frame_count: u32,
}

impl Interconnect {
    pub fn default() -> Self {
        Self {
            cpu: Cpu::default(),
            frame_count: 0,
        }
    }

    pub fn execute_cpu(&mut self) -> u32 {
        // self.cpu.debug = true;
        let mut cycles_executed: usize = 0;
        // Cycles per frame should be: 3072000
        // Divide amount of cycles per frame with 60 FPS
        // Divide that by 2 to get half cycles per frame (for interrupts)

        while cycles_executed <= 25_600 {
            let start_cycles = self.cpu.cycles;
            self.cpu.execute();

            cycles_executed += self.cpu.cycles - start_cycles;
            self.cpu.poll_interrupt();
        }

        self.frame_count += 1;
        self.frame_count
    }

    pub fn run_tests(&mut self) {
        self.cpu.fetch();
        if self.cpu.debug {
            // self.debug_decode();
            println!("{:#?}", self.cpu);
        }
        self.cpu.decode(self.cpu.opcode);
    }
    fn debug_decode(&mut self) {
        self.cpu.instruction = Instruction::decode(&mut self.cpu)
            .expect(format!("Unknown opcode:{:04X}", self.cpu.opcode).as_str());

        if self.cpu.instruction.name.to_string().len() < 1 {
            self.cpu.current_instruction = format!("{:w$}", self.cpu.current_instruction, w = 12);
        } else {
            self.cpu.current_instruction = self.cpu.instruction.name.to_string();
        }
        println!("{:#?}", self.cpu);
    }
}
