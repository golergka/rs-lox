use crate::value::*;
use crate::rle::*;
use std::convert::TryInto;

pub const OP_CONSTANT: u8 = 0;
pub const OP_RETURN: u8 = 1;

pub struct Chunk {
    pub code: Vec<u8>,
    constants: ValueArray,
    lines: Rle<u16>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk {
            code: Vec::new(),
            constants: ValueArray::new(),
            lines: Rle::new(),
        }
    }

    pub fn write_chunk(&mut self, op: u8, line: u16) {
        self.code.push(op);
        self.lines.push(line);
    }
    
    pub fn get_line(&self, offset: usize) -> Option<&u16> {
        self.lines.get(offset)
    }

    pub fn add_constant(&mut self, value: Value) -> u8 {
        self.constants.push(value);
        return (self.constants.len() - 1).try_into().unwrap();
    }
    
    pub fn get_constant(&self, offset: usize) -> Value {
        self.constants[offset]
    }
}