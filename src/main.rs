mod chunk;
mod debug;

use std::error::Error;
use chunk::*;
use debug::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut chunk = Chunk::new();
    chunk.push(OpCode::OP_RETURN);
    disassemble_chunk(&chunk, "test chunk");
    Ok(())
}
