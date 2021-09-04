use crate::chunk::*;

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
    let mut result = String::new();
    result.push_str(&format!("{:04} ", offset));
    let instruction = chunk.code.get(offset)?;
    return match instruction {
        OpCode::OpReturn => {
            let (offset, description) = simple_instruction("OP_RETURN", offset);
            result.push_str(&description);
            Some((offset, result))
        }
    }
}

fn simple_instruction(name: &str, offset: usize) -> (usize, String) {
    return (offset + 1, format!("{}", name));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn simple_return() {
        let mut chunk = Chunk::new();
        chunk.code.push(OpCode::OpReturn);
        let result = disassemble_chunk(&chunk, "test chunk");
        assert_eq!(result, String::from(
            "== test chunk ==\n\
            0000 OP_RETURN\n"
        ));
    }

}