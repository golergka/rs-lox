use crate::chunk::*;
use crate::debug::*;
use crate::value::*;

pub struct VMConfig {
    pub trace_execution: bool,
}

pub struct VM<'a> {
    chunk: &'a Chunk,
    ip: usize,
    config: VMConfig,
}

pub enum InterpreterError {
    RuntimeError(String),
    CompileError,
}

pub type InterpreterResult = Result<(), InterpreterError>;

impl<'a> VM<'a> {
    pub fn new(config: VMConfig, chunk: &'a Chunk) -> Self {
        VM {
            chunk,
            ip: 0,
            config,
        }
    }

    pub fn interpret(&mut self, chunk: &'a Chunk) -> InterpreterResult {
        self.chunk = chunk;
        self.ip = 0;
        return self.run();
    }

    fn read_byte(&mut self) -> u8 {
        let b = self.chunk.get_code()[self.ip];
        self.ip += 1;
        return b;
    }

    fn read_short(&mut self) -> u16 {
        let s = self.chunk.read_short(self.ip);
        self.ip += 2;
        return s;
    }

    fn read_constant(&mut self) -> Value {
        self.chunk.get_constant(self.read_byte() as usize)
    }

    fn read_constant_long(&mut self) -> Value {
        self.chunk.get_constant(self.read_short() as usize)
    }

    pub fn run(&mut self) -> InterpreterResult {
        loop {
            if self.config.trace_execution {
                if let Some((_, decription)) = disassemble_instruction(self.chunk, self.ip) {
                    println!("{}", decription);
                } else {
                    println!("Unknown opcode: {}", self.chunk.get_code()[self.ip]);
                }
            }
            let instruction = self.read_byte();
            match instruction {
                OP_RETURN => return Ok(()),
                OP_CONSTANT => {
                    let constant = self.read_constant();
                    println!("{}", constant);
                    return Ok(());
                }
                OP_CONSTANT_LONG => {
                    let constant = self.read_constant_long();
                    println!("{}", constant);
                    return Ok(());
                }
                _ => {
                    return Err(InterpreterError::RuntimeError(format!(
                        "Unknown opcode: {}",
                        instruction
                    )))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_return() {
        let mut chunk = Chunk::new();
        chunk.write_chunk(OP_RETURN, 1);
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
            },
            &chunk,
        );
        let result = vm.interpret(&chunk);
        assert!(result.is_ok());
    }
    #[test]
    fn simple_constant() {
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1);
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
            },
            &chunk,
        );
        let result = vm.interpret(&chunk);
        assert!(result.is_ok());
    }
    #[test]
    fn simple_constant_long() {
        let mut chunk = Chunk::new();
        for i in 0..300 {
            chunk.write_constant(i as f32, i);
        }
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
            },
            &chunk,
        );
        let result = vm.interpret(&chunk);
        assert!(result.is_ok());
    }
}
