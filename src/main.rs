mod chunk;
mod debug;
mod value;

use std::error::Error;
use chunk::*;
use debug::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut chunk = Chunk::new();
    let constant = chunk.add_constant(1.2);
    chunk.write_chunk(OP_CONSTANT, 1);
    chunk.write_chunk(constant, 1);
    print!("{}", disassemble_chunk(&chunk, "test chunk"));
    Ok(())
}
