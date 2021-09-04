mod chunk;
mod debug;
mod value;

use std::error::Error;
use chunk::*;
use debug::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut chunk = Chunk::new();
    chunk.code.push(OpCode::OpReturn);
    print!("{}", disassemble_chunk(&chunk, "test chunk"));
    Ok(())
}
