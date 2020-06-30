pub mod cpu;
pub mod cpu_tests;
pub mod formatter;
pub mod instruction_info;
pub mod interconnect;
pub mod memory;

/*
pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut ctx = Interconnect::new();
    ctx.cpu.memory.load_bin(&args);

    loop {
        // std::io::stdin().read_line(&mut String::new()).unwrap();
        ctx.execute_cpu();
    }
}*/
