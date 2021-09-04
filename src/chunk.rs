use crate::value::*;

pub enum OpCode {
    OpReturn
}

pub struct Chunk {
    pub code: Vec<OpCode>,
    pub constants: ValueArray
}

impl Chunk {

    pub fn new() -> Chunk {
        Chunk {
            code: Vec::new(),
            constants: ValueArray::new()
        }
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        return self.constants.len() - 1;
    }
}