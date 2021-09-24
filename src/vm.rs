use crate::chunk::*;
use crate::compiler::ParserError;
use crate::debug::*;
use crate::value::{are_equal, is_falsey, Value, Value::*};
use crate::vm::OpCode::*;
use crate::InterpreterError::*;
use num_traits::FromPrimitive;
use std::fmt;
use std::fmt::Formatter;
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        return match self {
            RuntimeError(s) => write!(f, "Runtime error: {}", s),
            CompileError(_) => write!(f, "Compile error"),
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
                RuntimeError(
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
            stack: [Value::Nil; STACK_MAX],
            stack_top: 0,
        }
    }
    fn stack_push(&mut self, value: Value) -> Result<(), InterpreterError> {
        if self.stack_top == STACK_MAX {
            return Err(RuntimeError(format!("Stack overflow")));
        }
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
        return Ok(());
    }
    fn stack_pop(&mut self) -> Result<Value, InterpreterError> {
        if self.stack_top == 0 {
            return Err(RuntimeError(format!("Stack underflow")));
        }
        self.stack_top -= 1;
        return Ok(self.stack[self.stack_top]);
    }
    fn stack_pop_binary(&mut self) -> Result<(Value, Value), InterpreterError> {
        let b = self.stack_pop()?;
        let a = self.stack_pop()?;
        Ok((a, b))
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
            .ok_or(RuntimeError(format!("Read byte out of bounds")));
        if b.is_ok() {
            self.ip += 1;
        }
        return b;
    }

    fn read_short(&mut self) -> Result<u16, InterpreterError> {
        let s = self
            .chunk
            .read_short(self.ip)
            .ok_or(RuntimeError(format!("Read short out of bounds")));
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
            vm_print!(self, "[{}]", self.stack[i]);
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
            let instruction = FromPrimitive::from_u8(byte)
                .ok_or(RuntimeError(format!("Unknown opcode: {}", byte)))?;
            match instruction {
                Return => {
                    let value = self.stack_pop()?;
                    vm_print!(self, "{}\n", value);
                    return Ok(value);
                }
                Constant => {
                    let constant = self.read_constant()?;
                    self.stack_push(constant)?;
                }
                ConstantLong => {
                    let constant = self.read_constant_long()?;
                    self.stack_push(constant)?;
                }
                OpCode::Nil => {
                    self.stack_push(Value::Nil)?;
                }
                True => {
                    self.stack_push(Boolean(true))?;
                }
                False => {
                    self.stack_push(Boolean(false))?;
                }
                Equal => {
                    let (a, b) = self.stack_pop_binary()?;
                    self.stack_push(Boolean(are_equal(a, b)))?;
                }
                Less => {
                    let (a, b) = self.stack_pop_binary()?;
                    self.stack_push(Boolean(a < b))?;
                }
                Greater => {
                    let (a, b) = self.stack_pop_binary()?;
                    self.stack_push(Boolean(a > b))?;
                }
                Add => {
                    let (a, b) = self.stack_pop_binary()?;
                    if let (Number(a), Number(b)) = (a, b) {
                        self.stack_push(Number(a + b))?;
                    } else {
                        return Err(RuntimeError(format!(
                            "Invalid type for addition: {} {}",
                            a, b
                        )));
                    }
                }
                Subtract => {
                    let (a, b) = self.stack_pop_binary()?;
                    if let (Number(a), Number(b)) = (a, b) {
                        self.stack_push(Number(a - b))?;
                    } else {
                        return Err(RuntimeError(format!(
                            "Invalid type for subtraction: {} {}",
                            a, b
                        )));
                    }
                }
                Multiply => {
                    let (a, b) = self.stack_pop_binary()?;
                    if let (Number(a), Number(b)) = (a, b) {
                        self.stack_push(Number(a * b))?;
                    } else {
                        return Err(RuntimeError(format!(
                            "Invalid type for multiplication: {} {}",
                            a, b
                        )));
                    }
                }
                Divide => {
                    let (a, b) = self.stack_pop_binary()?;
                    if let (Number(a), Number(b)) = (a, b) {
                        self.stack_push(Number(a / b))?;
                    } else {
                        return Err(RuntimeError(format!(
                            "Invalid type for division: {} {}",
                            a, b
                        )));
                    }
                }
                Negate => {
                    let value = self.stack_pop()?;
                    if let Number(n) = value {
                        self.stack_push(Number(-n))?;
                    } else {
                        return Err(RuntimeError(format!(
                            "Invalid type for negation: {}",
                            value
                        )));
                    }
                }
                Not => {
                    let value = self.stack_pop()?;
                    self.stack_push(Value::Boolean(is_falsey(value)))?;
                }
            }
            self.config
                .stdout
                .flush()
                .map_err(|_| RuntimeError(format!("Failed to write to stdout")))?;
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
        chunk.write_opcode(Return, 1);
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
        assert_eq!(result, Err(RuntimeError(String::from("Stack underflow"))));
        assert_eq!(
            output,
            "          \n\
            0000    1 OP_RETURN\n"
        );
    }

    #[test]
    fn constant_wo_return() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.2), 1);
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
            Err(RuntimeError(String::from("Read byte out of bounds")))
        );
    }

    #[test]
    fn constants_break_stack() {
        let mut chunk = Chunk::new();
        for i in 0..257 {
            chunk.write_constant(Number(i as f32), i);
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
        assert_eq!(result, Err(RuntimeError(String::from("Stack overflow"))));
    }

    #[test]
    fn return_w_number_constant() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.2), 1);
        chunk.write_opcode(Return, 2);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(Number(1.2)));
    }

    #[test]
    fn return_w_bool_constant() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Boolean(true), 1);
        chunk.write_opcode(Return, 2);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(Boolean(true)));
    }

    #[test]
    fn return_w_nil_constant() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Nil, 1);
        chunk.write_opcode(Return, 2);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(Nil));
    }

    #[test]
    fn return_w_many_constants() {
        let mut chunk = Chunk::new();
        for i in 0..256 {
            chunk.write_constant(Number(i as f32), i);
        }
        chunk.write_opcode(Return, 256);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(Number(255.0)));
    }

    #[test]
    fn add() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.0), 1);
        chunk.write_constant(Number(1.0), 2);
        chunk.write_opcode(Add, 3);
        chunk.write_opcode(Return, 4);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(Number(2.0)));
    }

    #[test]
    fn add_nil() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Nil, 1);
        chunk.write_constant(Number(1.0), 2);
        chunk.write_opcode(Add, 3);
        chunk.write_opcode(Return, 4);
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
            Err(RuntimeError(String::from(
                "Invalid type for addition: nil 1"
            )))
        );
    }

    #[test]
    fn subtract() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.2), 1);
        chunk.write_constant(Number(3.4), 2);
        chunk.write_opcode(Subtract, 3);
        chunk.write_opcode(Return, 4);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(Number(-2.2)));
    }

    #[test]
    fn subtract_nil() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Nil, 1);
        chunk.write_constant(Number(1.0), 2);
        chunk.write_opcode(Subtract, 3);
        chunk.write_opcode(Return, 4);
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
            Err(RuntimeError(String::from(
                "Invalid type for subtraction: nil 1"
            )))
        );
    }

    #[test]
    fn multiply() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(3.0), 1);
        chunk.write_constant(Number(-0.5), 2);
        chunk.write_opcode(Multiply, 3);
        chunk.write_opcode(Return, 4);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(Number(-1.5)));
    }

    #[test]
    fn multiply_nil() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Nil, 1);
        chunk.write_constant(Number(1.0), 2);
        chunk.write_opcode(Multiply, 3);
        chunk.write_opcode(Return, 4);
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
            Err(RuntimeError(String::from(
                "Invalid type for multiplication: nil 1"
            )))
        );
    }

    #[test]
    fn divide() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(10.0), 1);
        chunk.write_constant(Number(2.0), 2);
        chunk.write_opcode(Divide, 3);
        chunk.write_opcode(Return, 4);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(Number(5.0)));
    }

    #[test]
    fn divide_nil() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Nil, 1);
        chunk.write_constant(Number(1.0), 2);
        chunk.write_opcode(Divide, 3);
        chunk.write_opcode(Return, 4);
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
            Err(RuntimeError(String::from(
                "Invalid type for division: nil 1"
            )))
        );
    }

    #[test]
    fn negate() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.2), 1);
        chunk.write_opcode(Negate, 2);
        chunk.write_opcode(Return, 3);
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
        assert_eq!(result, Ok(Number(-1.2)));
        assert_eq!(output, "-1.2\n");
    }

    #[test]
    fn negate_nil() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Nil, 1);
        chunk.write_opcode(Negate, 2);
        chunk.write_opcode(Return, 3);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: false,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(
            result,
            Err(RuntimeError(String::from("Invalid type for negation: nil")))
        );
    }

    #[test]
    fn not() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Boolean(true), 1);
        chunk.write_opcode(Not, 2);
        chunk.write_opcode(Return, 3);
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
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "false\n");
    }

    #[test]
    fn not_nil() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Nil, 1);
        chunk.write_opcode(Not, 2);
        chunk.write_opcode(Return, 3);
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
        assert_eq!(result, Ok(Boolean(true)));
        assert_eq!(output, "true\n");
    }
    #[test]
    fn not_zero() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(0.0), 1);
        chunk.write_opcode(Not, 2);
        chunk.write_opcode(Return, 3);
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
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "false\n");
    }
    #[test]
    fn not_one() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.0), 1);
        chunk.write_opcode(Not, 2);
        chunk.write_opcode(Return, 3);
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
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "false\n");
    }
    #[test]
    fn equal_true() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.0), 1);
        chunk.write_constant(Number(1.0), 2);
        chunk.write_opcode(Equal, 3);
        chunk.write_opcode(Return, 4);
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
        assert_eq!(result, Ok(Boolean(true)));
        assert_eq!(output, "true\n");
    }
    #[test]
    fn equal_false() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.0), 1);
        chunk.write_constant(Number(2.0), 2);
        chunk.write_opcode(Equal, 3);
        chunk.write_opcode(Return, 4);
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
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "false\n");
    }

    #[test]
    fn greater_true() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(2.0), 2);
        chunk.write_constant(Number(1.0), 1);
        chunk.write_opcode(Greater, 3);
        chunk.write_opcode(Return, 4);
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
        assert_eq!(result, Ok(Boolean(true)));
        assert_eq!(output, "true\n");
    }
    #[test]
    fn greater_false() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.0), 1);
        chunk.write_constant(Number(1.0), 2);
        chunk.write_opcode(Greater, 3);
        chunk.write_opcode(Return, 4);
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
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "false\n");
    }

    #[test]
    fn less_true() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.0), 1);
        chunk.write_constant(Number(2.0), 2);
        chunk.write_opcode(Less, 3);
        chunk.write_opcode(Return, 4);
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
        assert_eq!(result, Ok(Boolean(true)));
        assert_eq!(output, "true\n");
    }

    #[test]
    fn less_false() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.0), 1);
        chunk.write_constant(Number(1.0), 2);
        chunk.write_opcode(Less, 3);
        chunk.write_opcode(Return, 4);
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
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "false\n");
    }

    #[test]
    fn simple_operations() {
        let mut chunk = Chunk::new();
        chunk.write_constant(Number(1.2), 1);
        chunk.write_constant(Number(3.4), 1);
        chunk.write_opcode(Add, 1);
        chunk.write_constant(Number(5.6), 1);
        chunk.write_opcode(Divide, 1);
        chunk.write_opcode(Negate, 1);
        chunk.write_opcode(Return, 1);
        let mut adapter = PrintAdapter {};
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut adapter,
            },
            &chunk,
        );
        let result = vm.interpret_chunk(&chunk);
        assert_eq!(result, Ok(Number(-0.82142866)));
    }
}
