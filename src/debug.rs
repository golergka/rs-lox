use crate::chunk::*;
use crate::value::*;
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
    let instruction = chunk.get_code()[offset];
    let (new_offset, instr_description): (usize, String) = match instruction {
        OP_CONSTANT => constant_instruction("OP_CONSTANT", chunk, offset)?,
        OP_CONSTANT_LONG => constant_long_instruction("OP_CONSTANT_LONG", chunk, offset)?,
        OP_RETURN => simple_instruction("OP_RETURN", offset),
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
    let constant = chunk.get_code()[offset + 1];
    let index: usize = constant.try_into().ok()?;
    let value: f32 = chunk.get_constant(index);
    let description = format!("{} {} '{}'", name, constant, print_value(value));
    return Some((offset + 2, description));
}

fn constant_long_instruction(name: &str, chunk: &Chunk, offset: usize) -> Option<(usize, String)> {
    let constant = chunk.read_short(offset + 1);
    let index: usize = constant.try_into().ok()?;
    let value: f32 = chunk.get_constant(index);
    let description = format!("{} {} '{}'", name, constant, print_value(value));
    return Some((offset + 3, description));
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
        chunk.write_constant(1.2, 1);
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
            chunk.write_constant(i as f32, i);
        }
        let mut target_result = String::from("== test chunk ==\n");
        for i in 0..256 {
            target_result.push_str(&format!(
                "{:04} {:4} OP_CONSTANT {} '{}'\n",
                i * 2,
                i,
                i,
                print_value(i as f32)
            ));
        }
        for i in 256..300 {
            target_result.push_str(&format!(
                "{:04} {:4} OP_CONSTANT_LONG {} '{}'\n",
                509 + 3 * (i - 255),
                i,
                i,
                print_value(i as f32)
            ));
        }
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(result, target_result);
    }

    #[test]
    fn line_numbers() {
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 123);
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
