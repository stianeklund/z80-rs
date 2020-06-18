use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::ops::{Index, IndexMut};
use std::path::Path;

pub struct Memory {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
}

impl fmt::Debug for Memory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = self;
        write!(f, "{:?}", val)
    }
}

impl fmt::UpperHex for Memory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val = self;
        write!(f, "{:04X}", val)
    }
}

impl IndexMut<u16> for Memory {
    fn index_mut(&mut self, index: u16) -> &mut u8 {
        &mut self.rom[index as usize]
    }
}

impl Index<u16> for Memory {
    type Output = u8;
    fn index(&self, index: u16) -> &u8 {
        &self.rom[index as usize]
    }
}

pub trait MemoryRW {
    fn read8(&self, addr: u16) -> u8;
    fn read16(&self, addr: u16) -> u16;
    fn write16(&mut self, addr: u16, word: u16);
    fn write8(&mut self, addr: u16, byte: u8);
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            rom: vec![0; 0x1_5000],
            ram: vec![0; 0x1_0000],
        }
    }
    pub fn peek(&self, v: usize) -> u8 {
        self.rom[v]
    }

    pub fn load_bin(&mut self, rom: &Vec<String>) {
        let mut buf = Vec::new();
        let mut collection: Vec<&str> = Vec::new();

        for i in rom.iter().skip(1) {
            collection.push(&i);
        }

        for f in collection.iter() {
            let path = Path::new(f);
            let mut file = File::open(&path).unwrap();
            file.read_to_end(&mut buf).expect("Failed to read binary");
            // self.rom[..buf.len()].clone_from_slice(&buf[..]);

            for i in 0..buf.len() {
                self.rom[i] = buf[i];
            }
            println!("Loaded: {:?} Bytes: {:?}", path, buf.len());
        }
    }

    pub fn load_tests(&mut self, file: &str) {
        let path = Path::new(file);
        let mut file = File::open(&path).expect(&*format!("Couldn't load binary file {:?}", path));
        let mut buf = Vec::new();

        file.read_to_end(&mut buf).expect("Failed to read binary");
        // Tests are loaded at 0x0100
        self.rom[0x0100..(buf.len() + 0x0100)].clone_from_slice(&buf[..]);
        println!("Test loaded: {:?} Bytes: {:?}\n", path, buf.len());
    }
}
