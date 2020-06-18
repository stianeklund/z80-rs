use crate::interconnect::Interconnect;

mod cpu;
mod formatter;
mod instruction_info;
mod interconnect;
mod memory;
mod tests;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut ctx = Interconnect::new();
    ctx.cpu.memory.load_bin(&args);

    loop {
        // std::io::stdin().read_line(&mut String::new()).unwrap();
        ctx.execute_cpu();
        /*if i.frame_count % 5 == 1 {
            i.keypad.reset_ports(&mut i.cpu.io);
        }*/
    }
}
