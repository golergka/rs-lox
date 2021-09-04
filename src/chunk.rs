use std::convert::TryInto;
use crate::value::*;

pub const OP_CONSTANT: u8 = 0;
pub const OP_RETURN: u8 = 1;

pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: ValueArray
}

impl Chunk {

    pub fn new() -> Chunk {
        Chunk {
            code: Vec::new(),
            constants: ValueArray::new()
        }
    }

    pub fn write_chunk(&mut self, op: u8) {
        self.code.push(op);
    }

    pub fn add_constant(&mut self, value: Value) -> u8 {
        self.constants.push(value);
        return (self.constants.len() - 1).try_into().unwrap();
    }
}