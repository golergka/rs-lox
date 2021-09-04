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
    let instruction = chunk.code.get(offset)?;
    let (new_offset, instr_description): (usize, String) = match instruction {
        &OP_CONSTANT => constant_instruction("OP_CONSTANT", chunk, offset)?,
        &OP_RETURN => simple_instruction("OP_RETURN", offset),
        _ => (offset + 1, format!("Unknown opcode {:?}", instruction)),
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
    let constant = *chunk.code.get(offset + 1)?;
    let index: usize = constant.try_into().ok()?;
    let value: f32 = chunk.get_constant(index);
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
                0000    1 OP_RETURN\n"
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
                0000    1 OP_CONSTANT 0 '1.2'\n"
            )
        );
    }

    #[test]
    fn line_numbers() {
        let mut chunk = Chunk::new();
        let constant = chunk.add_constant(1.2);
        chunk.write_chunk(OP_CONSTANT, 123);
        chunk.write_chunk(constant, 123);
        chunk.write_chunk(OP_RETURN, 123);
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
}
