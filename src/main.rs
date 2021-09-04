mod chunk;
mod debug;
mod value;
mod rle;

use std::error::Error;
use chunk::*;
use debug::*;

fn main() -> Result<(), Box<dyn Error>> {
    let mut chunk = Chunk::new();
    chunk.write_constant(1.2, 1);
    print!("{}", disassemble_chunk(&chunk, "test chunk"));
    Ok(())
}
