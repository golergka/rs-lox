mod chunk;
mod debug;
mod rle;
mod value;
mod vm;

use chunk::*;
use debug::*;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let mut chunk = Chunk::new();
    chunk.write_constant(1.2, 1);
    print!("{}", disassemble_chunk(&chunk, "test chunk"));
    Ok(())
}
