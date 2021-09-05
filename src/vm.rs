use crate::chunk::*;
use crate::debug::*;
use crate::value::*;

#[derive(Debug)]
pub struct VMConfig {
    pub trace_execution: bool,
}

pub const STACK_MAX: usize = 256;

pub struct VM<'a> {
    chunk: &'a Chunk,
    ip: usize,
    config: VMConfig,
    stack: [Value; STACK_MAX],
    stack_top: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InterpreterError {
    RuntimeError(String),
}

pub type InterpreterResult = Result<(), InterpreterError>;

impl<'a> VM<'a> {
    pub fn new(config: VMConfig, chunk: &'a Chunk) -> Self {
        VM {
            chunk,
            ip: 0,
            config,
            stack: [0.0; STACK_MAX],
            stack_top: 0,
        }
    }
    fn stack_push(&mut self, value: Value) -> Result<(), InterpreterError> {
        if self.stack_top == STACK_MAX {
            return Err(InterpreterError::RuntimeError(format!("Stack overflow")));
        }
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
        return Ok(());
    }
    fn stack_pop(&mut self) -> Result<Value, InterpreterError> {
        if self.stack_top == 0 {
            return Err(InterpreterError::RuntimeError(format!("Stack underflow")));
        }
        self.stack_top -= 1;
        return Ok(self.stack[self.stack_top]);
    }

    pub fn interpret(&mut self, chunk: &'a Chunk) -> InterpreterResult {
        self.chunk = chunk;
        self.ip = 0;
        return self.run();
    }

    fn read_byte(&mut self) -> Result<u8, InterpreterError> {
        let b = self
            .chunk
            .read_byte(self.ip)
            .ok_or(InterpreterError::RuntimeError(format!(
                "Read byte out of bounds"
            )));
        if b.is_ok() {
            self.ip += 1;
        }
        return b;
    }

    fn read_short(&mut self) -> Result<u16, InterpreterError> {
        let s = self
            .chunk
            .read_short(self.ip)
            .ok_or(InterpreterError::RuntimeError(format!(
                "Read short out of bounds"
            )));
        if s.is_ok() {
            self.ip += 2;
        }
        return s;
    }

    fn read_constant(&mut self) -> Result<Value, InterpreterError> {
        let b = self.read_byte()?;
        return Ok(self.chunk.get_constant(b as usize));
    }

    fn read_constant_long(&mut self) -> Result<Value, InterpreterError> {
        let s = self.read_short()?;
        return Ok(self.chunk.get_constant(s as usize));
    }

    fn trace_instruction(&self) {
        print!("          ");
        for i in 0..self.stack_top {
            print!("[{}]", print_value(self.stack[i]));
        }
        print!("\n");
        if let Some((_, decription)) = disassemble_instruction(self.chunk, self.ip) {
            println!("{}", decription);
        } else {
            println!("[END OF CHUNK]");
        }
    }

    pub fn run(&mut self) -> InterpreterResult {
        loop {
            if self.config.trace_execution {
                self.trace_instruction();
            }
            let instruction = self.read_byte()?;
            match instruction {
                OP_RETURN => {
                    let value = self.stack_pop()?;
                    println!("{}", print_value(value));
                    return Ok(());
                }
                OP_CONSTANT => {
                    let constant = self.read_constant()?;
                    self.stack_push(constant)?;
                }
                OP_CONSTANT_LONG => {
                    let constant = self.read_constant_long()?;
                    self.stack_push(constant)?;
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
    fn return_wo_constant() {
        let mut chunk = Chunk::new();
        chunk.write_chunk(OP_RETURN, 1);
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
            },
            &chunk,
        );
        let result = vm.interpret(&chunk);
        assert_eq!(
            result,
            Err(InterpreterError::RuntimeError(String::from(
                "Stack underflow"
            )))
        );
    }

    #[test]
    fn constant_wo_return() {
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1);
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
            },
            &chunk,
        );
        let result = vm.interpret(&chunk);
        assert_eq!(
            result,
            Err(InterpreterError::RuntimeError(String::from(
                "Read byte out of bounds"
            )))
        );
    }

    #[test]
    fn constants_break_stack() {
        let mut chunk = Chunk::new();
        for i in 0..257 {
            chunk.write_constant(i as f32, i);
        }
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
            },
            &chunk,
        );
        let result = vm.interpret(&chunk);
        assert_eq!(
            result,
            Err(InterpreterError::RuntimeError(String::from(
                "Stack overflow"
            )))
        );
    }

    #[test]
    fn return_w_constant() {
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1);
        chunk.write_chunk(OP_RETURN, 2);
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
