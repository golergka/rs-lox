use crate::chunk::*;

pub fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);
    let mut offset = 0;
    while offset < chunk.len() {
        let next_offset = disassemble_instruction(chunk, offset);
        match next_offset {
            None => break,
            Some(next_offset) => offset = next_offset,
        }
    }
}

fn disassemble_instruction(chunk: &Chunk, offset: usize) -> Option<usize> {
    print!("{:04} ", offset);
    let instruction = chunk.get(offset)?;
    return match instruction {
        OpCode::OP_RETURN => Some(simple_instruction("OP_RETURN", offset))
    }
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    return offset + 1;
}