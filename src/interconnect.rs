use super::cpu::Cpu;

pub struct Interconnect {
    pub cpu: Cpu,
    pub frame_count: u32,
}

impl Interconnect {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::new(),
            frame_count: 0,
        }
    }

    pub fn execute_cpu(&mut self) -> u32 {
        let vblank: bool = false;
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
            if vblank {
                self.cpu.generate_interrupt();
            }
        }

        self.frame_count += 1;
        return self.frame_count;
    }

    pub fn run_tests(&mut self) {
        self.cpu.execute();
    }
}
