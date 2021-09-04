use crate::chunk::*;
use crate::value::*;
use std::convert::TryInto;

pub fn disassemble_chunk(chunk: &Chunk, name: &str) -> String {
    let mut result = String::new();
    result.push_str(&format!("== {} ==\n", name));
    let mut offset = 0;
    while offset < chunk.code.len() {
        let next_offset = disassemble_instruction(chunk, offset);
        match next_offset {
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

fn disassemble_instruction(chunk: &Chunk, offset: usize) -> Option<(usize, String)> {
    let mut description = String::new();
    description.push_str(&format!("{:04} ", offset));
    let instruction = chunk.code.get(offset)?;
    return match instruction {
        &OP_CONSTANT => {
            let (offset, instr_description) = constant_instruction("OP_CONSTANT", chunk, offset)?;
            description.push_str(&instr_description);
            Some((offset, description))
        }
        &OP_RETURN => {
            let (offset, instr_description) = simple_instruction("OP_RETURN", offset);
            description.push_str(&instr_description);
            Some((offset, description))
        }
        _ => {
            description.push_str(&format!("Unknown opcode {:?}", instruction));
            Some((offset + 1, description))
        }
    };
}

fn simple_instruction(name: &str, offset: usize) -> (usize, String) {
    return (offset + 1, format!("{}", name));
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> Option<(usize, String)> {
    let constant = *chunk.code.get(offset + 1)?;
    let index: usize = constant.try_into().ok()?;
    let value: f32 = *chunk.constants.get(index)?;
    let description = format!("{} {} '{}'", name, constant, print_value(value));
    return Some((offset + 2, description));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn simple_return() {
        let mut chunk = Chunk::new();
        chunk.write_chunk(OP_RETURN, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000 OP_RETURN\n"
            )
        );
    }

    #[test]
    fn simple_constant() {
        let mut chunk = Chunk::new();
        let constant = chunk.add_constant(1.2);
        chunk.write_chunk(OP_CONSTANT, 1);
        chunk.write_chunk(constant, 1);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(
            result,
            String::from(
                "== test chunk ==\n\
                0000 OP_CONSTANT 0 '1.2'\n"
            )
        );
    }
}
