use crate::rle::*;
use crate::value::*;
use std::convert::TryFrom;

pub const OP_RETURN: u8 = 0;
pub const OP_CONSTANT: u8 = 1;
pub const OP_CONSTANT_LONG: u8 = 2;

pub type LineNumber = u16;

#[derive(Debug)]
pub struct Chunk {
    code: Vec<u8>,
    constants: ValueArray,
    lines: Rle<LineNumber>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk {
            code: Vec::new(),
            constants: ValueArray::new(),
            lines: Rle::new(),
        }
    }

    pub fn write_chunk(&mut self, op: u8, line: LineNumber) {
        self.code.push(op);
        self.lines.push(line);
    }

    pub fn write_short(&mut self, value: u16, line: LineNumber) {
        // LITTLE ENDIAN
        let [a, b] = value.to_be_bytes();
        self.code.push(a);
        self.code.push(b);
        self.lines.push(line);
        self.lines.push(line);
    }

    pub fn read_byte(&self, index: usize) -> Option<u8> {
        let a = self.code.get(index)?;
        return Some(*a);
    }

    pub fn read_short(&self, index: usize) -> Option<u16> {
        let a = self.code.get(index)?;
        let b = self.code.get(index + 1)?;
        return Some(u16::from_be_bytes([*a, *b]));
    }

    pub fn get_code(&self) -> &[u8] {
        &self.code
    }

    pub fn get_line(&self, offset: usize) -> Option<&LineNumber> {
        self.lines.get(offset)
    }

    fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        return self.constants.len() - 1;
    }

    pub fn write_constant(&mut self, value: Value, line: LineNumber) {
        let constant = self.add_constant(value);
        if let Ok(op) = u8::try_from(constant) {
            self.write_chunk(OP_CONSTANT, line);
            self.write_chunk(op, line);
        } else if let Ok(op) = u16::try_from(constant) {
            self.write_chunk(OP_CONSTANT_LONG, line);
            self.write_short(op, line);
        } else {
            panic!("Can't support more than 65 536 constants");
        }
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
    fn adds_correct_constant() {
        let mut chunk = Chunk::new();
        chunk.add_constant(1.2);
        assert_eq!(chunk.get_constant(0), 1.2);
    }

    #[test]
    fn writes_constant() {
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1);
        assert_eq!(chunk.get_code(), &[OP_CONSTANT, 0]);
        assert_eq!(chunk.get_constant(0), 1.2);
    }

    #[test]
    fn writes_300_constants() {
        let mut chunk = Chunk::new();
        for i in 0..300 {
            chunk.write_constant(i as f32, i);
        }
        for i in 0..300 {
            assert_eq!(chunk.get_constant(i), i as f32);
        }
    }
}
