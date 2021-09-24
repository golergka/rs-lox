use crate::compiler::ParserError;
use crate::chunk::*;
use crate::debug::*;
use crate::value::*;
use num_traits::FromPrimitive;
use std::fmt;
use std::io;

pub struct VMConfig<'a> {
    pub trace_execution: bool,
    pub stdout: &'a mut dyn io::Write,
}

impl std::fmt::Debug for VMConfig<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return f
            .debug_struct("VMConfig")
            .field("trace_execution", &self.trace_execution)
            .finish();
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum InterpreterError {
    CompileError(Vec<ParserError>),
    RuntimeError(String),
}

impl fmt::Display for InterpreterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        return match self {
            InterpreterError::RuntimeError(s) => write!(f, "Runtime error: {}", s),
            InterpreterError::CompileError(_) => write!(f, "Compile error"),
        };
    }
}

impl std::error::Error for InterpreterError {}

pub const STACK_MAX: usize = 256;

pub struct VM<'a> {
    chunk: &'a Chunk,
    ip: usize,
    config: VMConfig<'a>,
    stack: [Value; STACK_MAX],
    stack_top: usize,
}

macro_rules! vm_print {
    ($dst:expr, $($arg:tt)*) => (
        $dst
            .config
            .stdout
            .write_fmt(std::format_args!($($arg)*))
            .map_err(|_| {
                InterpreterError::RuntimeError(
                    format!("Failed to write to stdout")
                )
            })?
    );
}

impl<'a> VM<'a> {
    pub fn new(config: VMConfig<'a>, chunk: &'a Chunk) -> Self {
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
    pub fn interpret_chunk(&mut self, chunk: &'a Chunk) -> Result<Value, InterpreterError> {
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

    fn trace_instruction(&mut self) -> Result<(), InterpreterError> {
        vm_print!(self, "          ");
        for i in 0..self.stack_top {
            vm_print!(self, "[{}]", print_value(self.stack[i]));
        }
        vm_print!(self, "\n");
        if let Some((_, decription)) = disassemble_instruction(self.chunk, self.ip) {
            vm_print!(self, "{}\n", decription);
        } else {
            vm_print!(self, "[END OF CHUNK]\n");
        }
        return Ok(());
    }

    pub fn run(&mut self) -> Result<Value, InterpreterError> {
        loop {
            if self.config.trace_execution {
                self.trace_instruction()?;
            }
            let byte = self.read_byte()?;
            let instruction = FromPrimitive::from_u8(byte).ok_or(
                InterpreterError::RuntimeError(format!("Unknown opcode: {}", byte)),
            )?;
            match instruction {
                OpCode::Return => {
                    let value = self.stack_pop()?;
                    vm_print!(self, "{}\n", print_value(value));
                    return Ok(value);
                }
                OpCode::Constant => {
                    let constant = self.read_constant()?;
                    self.stack_push(constant)?;
                }
                OpCode::ConstantLong => {
                    let constant = self.read_constant_long()?;
                    self.stack_push(constant)?;
                }
                OpCode::Negate => {
                    let value = self.stack_pop()?;
                    self.stack_push(-value)?;
                }
                OpCode::Add => {
                    let b = self.stack_pop()?;
                    let a = self.stack_pop()?;
                    self.stack_push(a + b)?;
                }
                OpCode::Subtract => {
                    let b = self.stack_pop()?;
                    let a = self.stack_pop()?;
                    self.stack_push(a - b)?;
                }
                OpCode::Multiply => {
                    let b = self.stack_pop()?;
                    let a = self.stack_pop()?;
                    self.stack_push(a * b)?;
                }
                OpCode::Divide => {
                    let b = self.stack_pop()?;
                    let a = self.stack_pop()?;
                    self.stack_push(a / b)?;
                }
            }
            self.config.stdout.flush().map_err(|_| {
                InterpreterError::RuntimeError(format!("Failed to write to stdout"))
            })?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str;

    struct StringAdapter<'a> {
        f: &'a mut String,
    }

    impl<'a> io::Write for StringAdapter<'a> {
        fn write(&mut self, b: &[u8]) -> Result<usize, io::Error> {
            use std::fmt::Write;
            let s = str::from_utf8(b).map_err(|_| io::Error::from(io::ErrorKind::Other))?;
            self.f
                .write_str(s)
                .map_err(|_| io::Error::from(io::ErrorKind::Other))?;
            Ok(b.len())
        }

        fn flush(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
    }

    struct PrintAdapter {}

    impl io::Write for PrintAdapter {
        fn write(&mut self, b: &[u8]) -> Result<usize, io::Error> {
            let s = str::from_utf8(b).map_err(|_| io::Error::from(io::ErrorKind::Other))?;
            print!("{}", s);
            Ok(b.len())
        }

        fn flush(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
    }

    #[test]
    fn return_wo_constant() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::Return, 1);
        let mut output = String::new();
        let mut adapter = StringAdapter { f: &mut output };
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(
            result,
            Err(InterpreterError::RuntimeError(String::from(
                "Stack underflow"
            )))
        );
        assert_eq!(
            output,
            "          \n\
            0000    1 OP_RETURN\n"
        );
    }

    #[test]
    fn constant_wo_return() {
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1);
        let mut adapter = PrintAdapter {};
        println!("test2");
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
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
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
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
        chunk.write_opcode(OpCode::Return, 2);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(1.2));
    }
    #[test]
    fn return_w_many_constants() {
        let mut chunk = Chunk::new();
        for i in 0..256 {
            chunk.write_constant(i as f32, i);
        }
        chunk.write_opcode(OpCode::Return, 256);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(255.0));
    }
    #[test]
    fn negate() {
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1);
        chunk.write_opcode(OpCode::Negate, 2);
        chunk.write_opcode(OpCode::Return, 3);
        let mut output = String::new();
        let mut adapter = StringAdapter { f: &mut output };
        let mut vm = VM::new(
            VMConfig {
                trace_execution: false,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(-1.2));
        assert_eq!(output, "-1.2\n");
    }
    #[test]
    fn add() {
        let mut chunk = Chunk::new();
        chunk.write_constant(1.0, 1);
        chunk.write_constant(1.0, 2);
        chunk.write_opcode(OpCode::Add, 3);
        chunk.write_opcode(OpCode::Return, 4);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(2.0));
    }
    #[test]
    fn subtract() {
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1);
        chunk.write_constant(3.4, 2);
        chunk.write_opcode(OpCode::Subtract, 3);
        chunk.write_opcode(OpCode::Return, 4);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(-2.2));
    }

    #[test]
    fn multiply() {
        let mut chunk = Chunk::new();
        chunk.write_constant(3.0, 1);
        chunk.write_constant(-0.5, 2);
        chunk.write_opcode(OpCode::Multiply, 3);
        chunk.write_opcode(OpCode::Return, 4);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(-1.5));
    }

    #[test]
    fn divide() {
        let mut chunk = Chunk::new();
        chunk.write_constant(10.0, 1);
        chunk.write_constant(2.0, 2);
        chunk.write_opcode(OpCode::Divide, 3);
        chunk.write_opcode(OpCode::Return, 4);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(5.0));
    }

    #[test]
    fn simple_operations() {
        let mut chunk = Chunk::new();
        chunk.write_constant(1.2, 1);
        chunk.write_constant(3.4, 1);
        chunk.write_opcode(OpCode::Add, 1);
        chunk.write_constant(5.6, 1);
        chunk.write_opcode(OpCode::Divide, 1);
        chunk.write_opcode(OpCode::Negate, 1);
        chunk.write_opcode(OpCode::Return, 1);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(-0.82142866));
    }
}
