use crate::rle::*;
use crate::value::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::convert::TryFrom;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, FromPrimitive)]
pub enum OpCode {
    Return,
    Constant,
    ConstantLong,
    Nil,
    True,
    False,
    Pop,
    Get,
    GetLong,
    DefineGlobal,
    DefineGlobalLong,
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Negate,
    Print,
}

pub type LineNumber = i16;

pub struct Chunk {
    code: Vec<u8>,
    constants: ValueArray,
    lines: Rle<LineNumber>,
}

impl fmt::Debug for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("code", &format_args!("{:#?}", self.code.iter().map(|b| OpCode::from_u8(*b).unwrap()).collect::<Vec<OpCode>>()))
            .field("constants", &self.constants)
            .field("lines", &self.lines)
            .finish()
    }
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk {
            code: Vec::new(),
            constants: ValueArray::new(),
            lines: Rle::new(),
        }
    }

    pub fn write_byte(&mut self, op: u8, line: LineNumber) {
        self.code.push(op);
        self.lines.push(line);
    }

    /// Helper method to write an OpCode, functionally equal to `write_byte`
    pub fn write_opcode(&mut self, op: OpCode, line: LineNumber) {
        self.write_byte(op as u8, line);
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

    pub fn add_const(&mut self, value: Value) -> usize {
        self.constants.push(value);
        return self.constants.len() - 1;
    }

    pub fn ref_const(
        &mut self,
        const_ref: usize,
        byte_op: OpCode,
        long_op: OpCode,
        line: LineNumber,
    ) {
        if let Ok(const_byte) = u8::try_from(const_ref) {
            self.write_opcode(byte_op, line);
            self.write_byte(const_byte, line);
        } else if let Ok(const_long) = u16::try_from(const_ref) {
            self.write_opcode(long_op, line);
            self.write_short(const_long, line);
        } else {
            panic!("Invalid constant reference");
        }
    }

    pub fn get_constant(&self, offset: usize) -> Value {
        match self.constants.get(offset) {
            Some(value) => value.clone(),
            None => panic!("Invalid constant reference: {}", offset),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn returns_correct_line() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::Constant, 1);
        chunk.write_opcode(OpCode::Return, 2);
        assert_eq!(chunk.get_line(0), Some(&1));
        assert_eq!(chunk.get_line(1), Some(&2));
    }

    #[test]
    fn returns_none_for_out_of_bounds_line() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::Constant, 1);
        chunk.write_opcode(OpCode::Return, 2);
        assert_eq!(chunk.get_line(2), None);
    }

    #[test]
    fn returns_correct_code() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::Constant, 1);
        chunk.write_opcode(OpCode::Return, 2);
        assert_eq!(
            chunk.get_code(),
            &[OpCode::Constant as u8, OpCode::Return as u8]
        );
    }

    #[test]
    fn adds_correct_constant() {
        let mut chunk = Chunk::new();
        chunk.add_const(Value::Number(1.2));
        assert_eq!(chunk.get_constant(0), Value::Number(1.2));
    }

    #[test]
    fn writes_constant() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Value::Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        assert_eq!(chunk.get_code(), &[OpCode::Constant as u8, 0]);
        assert_eq!(chunk.get_constant(0), Value::Number(1.2));
    }

    #[test]
    fn writes_300_constants() {
        let mut chunk = Chunk::new();
        for i in 0..300 {
            chunk.add_const(Value::Number(i as f32));
        }
        for i in 0..300 {
            assert_eq!(chunk.get_constant(i), Value::Number(i as f32));
        }
    }
}
