use crate::value::*;
use crate::rle::*;
use std::convert::TryInto;

pub const OP_CONSTANT: u8 = 0;
pub const OP_RETURN: u8 = 1;

pub struct Chunk {
    code: Vec<u8>,
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
    
    pub fn get_code(&self) -> &[u8] {
        &self.code
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn returns_correct_line() {
        let mut chunk = Chunk::new();
        chunk.write_chunk(OP_CONSTANT, 1);
        chunk.write_chunk(OP_RETURN, 2);
        assert_eq!(chunk.get_line(0), Some(&1));
        assert_eq!(chunk.get_line(1), Some(&2));
    }

    #[test]
    fn returns_none_for_out_of_bounds_line() {
        let mut chunk = Chunk::new();
        chunk.write_chunk(OP_CONSTANT, 1);
        chunk.write_chunk(OP_RETURN, 2);
        assert_eq!(chunk.get_line(2), None);
    }

    #[test]
    fn returns_correct_code() {
        let mut chunk = Chunk::new();
        chunk.write_chunk(OP_CONSTANT, 1);
        chunk.write_chunk(OP_RETURN, 2);
        assert_eq!(chunk.get_code(), &[OP_CONSTANT, OP_RETURN]);
    }

    #[test]
    fn returns_correct_constant() {
        let mut chunk = Chunk::new();
        chunk.add_constant(1.2);
        assert_eq!(chunk.get_constant(0), 1.2);
    }

}