use crate::chunk::OpCode::*;
use crate::chunk::*;
use crate::value::Value::Number;
use crate::value::*;
use num_traits::FromPrimitive;
use std::convert::TryInto;

pub fn disassemble_chunk(chunk: &Chunk, name: &str) -> String {
    let mut result = String::new();
    result.push_str(&format!("== {} ==\n", name));
    let mut offset = 0;
    while offset < chunk.get_code().len() {
        let instr_result = disassemble_instruction(chunk, offset);
        match instr_result {
            None => break,
            Some((next_offset, description)) => {
                result.push_str(&description);
                result.push_str("\n");
                offset = next_offset
            }
        }
    }
    return result;
}

pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> Option<(usize, String)> {
    let byte = chunk.read_byte(offset)?;
    let instruction: Option<OpCode> = FromPrimitive::from_u8(byte);
    let (new_offset, instr_description): (usize, String) = match instruction {
        Some(Return) => simple_instruction("OP_RETURN", offset),
        Some(Constant) => constant_instruction("OP_CONSTANT", chunk, offset)?,
        Some(ConstantLong) => constant_long_instruction("OP_CONSTANT_LONG", chunk, offset)?,
        Some(OpCode::Nil) => simple_instruction("OP_NIL", offset),
        Some(True) => simple_instruction("OP_TRUE", offset),
        Some(False) => simple_instruction("OP_FALSE", offset),
        Some(Pop) => simple_instruction("OP_POP", offset),
        Some(Get) => constant_instruction("OP_GET_GLOBAL", chunk, offset)?,
        Some(GetLong) => constant_long_instruction("OP_GET_GLOBAL_LONG", chunk, offset)?,
        Some(DefineGlobal) => constant_instruction("OP_DEFINE_GLOBAL", chunk, offset)?,
        Some(DefineGlobalLong) => {
            constant_long_instruction("OP_DEFINE_GLOBAL_LONG", chunk, offset)?
        }
        Some(Equal) => simple_instruction("OP_EQUAL", offset),
        Some(Greater) => simple_instruction("OP_GREATER", offset),
        Some(Less) => simple_instruction("OP_LESS", offset),
        Some(Add) => simple_instruction("OP_ADD", offset),
        Some(Subtract) => simple_instruction("OP_SUBTRACT", offset),
        Some(Multiply) => simple_instruction("OP_MULTIPLY", offset),
        Some(Divide) => simple_instruction("OP_DIVIDE", offset),
        Some(Negate) => simple_instruction("OP_NEGATE", offset),
        Some(Not) => simple_instruction("OP_NOT", offset),
        Some(Print) => simple_instruction("OP_PRINT", offset),
        None => {
            return Some((
                offset + 1,
                format!("[{}] Unknown opcode {:02x}", offset, byte),
            ))
        }
    };
    return Some((
        new_offset,
        format!(
            "{:04} {}{}",
            offset,
            line_info(chunk, offset),
            instr_description
        ),
    ));
}

fn line_info(chunk: &Chunk, offset: usize) -> String {
    let cur_line = chunk.get_line(offset).unwrap();
    if offset > 0 && cur_line == chunk.get_line(offset - 1).unwrap() {
        return String::from("   | ");
    } else {
        return format!("{:4} ", cur_line);
    }
}

fn simple_instruction(name: &str, offset: usize) -> (usize, String) {
    return (offset + 1, format!("{}", name));
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> Option<(usize, String)> {
    let constant = chunk.get_code()[offset + 1];
    let index: usize = constant.try_into().ok()?;
    let value = chunk.get_constant(index);
    let description = format!("{} {} '{}'", name, constant, value);
    return Some((offset + 2, description));
}

fn constant_long_instruction(name: &str, chunk: &Chunk, offset: usize) -> Option<(usize, String)> {
    let constant = chunk.read_short(offset + 1)?;
    let index: usize = constant.try_into().ok()?;
    let value = chunk.get_constant(index);
    let description = format!("{} {} '{}'", name, constant, value);
    return Some((offset + 3, description));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn retrn() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Return, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_RETURN\n"
            )
        );
    }

    #[test]
    fn constant() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_CONSTANT 0 '1.2'\n"
            )
        );
    }

    #[test]
    fn long_constant() {
        let mut chunk = Chunk::new();
        for i in 0..300 {
            let const_ref = chunk.add_const(Number(i as f32));
            chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, i);
        }
        let mut target_result = String::from("== test chunk ==\n");
        for i in 0..256 {
            target_result.push_str(&format!(
                "{:04} {:4} OP_CONSTANT {} '{}'\n",
                i * 2,
                i,
                i,
                Number(i as f32)
            ));
        }
        for i in 256..300 {
            target_result.push_str(&format!(
                "{:04} {:4} OP_CONSTANT_LONG {} '{}'\n",
                509 + 3 * (i - 255),
                i,
                i,
                Number(i as f32)
            ));
        }
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(result, target_result);
    }

    #[test]
    fn nil() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Nil, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_NIL\n"
            )
        );
    }

    #[test]
    fn true_false() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(True, 1);
        chunk.write_opcode(False, 2);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_TRUE\n\
                0001    2 OP_FALSE\n"
            )
        );
    }

    #[test]
    fn equal() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Equal, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_EQUAL\n"
            )
        );
    }

    #[test]
    fn greater() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Greater, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_GREATER\n"
            )
        );
    }

    #[test]
    fn add() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Add, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_ADD\n"
            )
        );
    }
    #[test]
    fn subtract() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Subtract, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_SUBTRACT\n"
            )
        );
    }

    #[test]
    fn multiply() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Multiply, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_MULTIPLY\n"
            )
        );
    }

    #[test]
    fn divide() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Divide, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_DIVIDE\n"
            )
        );
    }

    #[test]
    fn line_numbers() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 123);
        chunk.write_opcode(Return, 123);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000  123 OP_CONSTANT 0 '1.2'\n\
                0002    | OP_RETURN\n"
            )
        );
    }

    #[test]
    fn negate() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Negate, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_NEGATE\n"
            )
        );
    }

    #[test]
    fn not() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Not, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000    1 OP_NOT\n"
            )
        );
    }
}
