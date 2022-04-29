use crate::chunk::*;
use crate::compiler::ParserError;
use crate::debug::*;
use crate::gc::{Obj, GC};
use crate::table::Table;
use crate::value::{are_equal, is_falsey, Value, Value::*};
use crate::vm::OpCode::*;
use crate::InterpreterError::*;
use num_traits::FromPrimitive;
use std::fmt;
use std::fmt::Formatter;
use std::io;

pub struct VMConfig<'a> {
    pub trace_instructions: bool,
    pub trace_stack: bool,
    pub stdout: &'a mut dyn io::Write,
}

impl std::fmt::Debug for VMConfig<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return f
            .debug_struct("VMConfig")
            .field("trace_execution", &self.trace_instructions)
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
    globals: Table<Value>,
    gc: &'a mut GC,
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
    pub fn new(config: VMConfig<'a>, chunk: &'a Chunk, gc: &'a mut GC) -> Self {
        VM {
            chunk,
            ip: 0,
            config,
            stack: [Value::Nil; STACK_MAX],
            stack_top: 0,
            globals: Table::new(),
            gc,
        }
    }

    pub fn with_gc<T>(&mut self, f: impl FnOnce(&mut GC) -> T) -> T {
        f(self.gc)
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
        if let Some((_, decription)) = disassemble_instruction(self.chunk, self.ip) {
            vm_print!(self, "{}\n", decription);
        } else {
            vm_print!(self, "[END OF CHUNK]\n");
        }
        return Ok(());
    }

    fn trace_stack(&mut self) -> Result<(), InterpreterError> {
        vm_print!(self, "Stack:");
        for i in 0..self.stack_top {
            vm_print!(self, "[{}]", self.stack[i]);
        }
        vm_print!(self, "\n");
        return Ok(());
    }

    pub fn run(&mut self) -> Result<Value, InterpreterError> {
        if self.config.trace_instructions {
            vm_print!(self, "Tracing execution:\n");
            vm_print!(self, "Offs Line Instruction\n");
        }
        loop {
            if self.config.trace_stack {
                self.trace_stack()?;
            }
            if self.config.trace_instructions {
                self.trace_instruction()?;
            }
            let byte = self.read_byte()?;
            let instruction = FromPrimitive::from_u8(byte)
                .ok_or(RuntimeError(format!("Unknown opcode: {}", byte)))?;
            match instruction {
                Return => match self.stack_pop() {
                    Ok(value) => return Ok(value),
                    Err(_) => return Ok(Value::Nil),
                },
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
                Pop => {
                    self.stack_pop()?;
                }
                Get => {
                    let name_val = self.read_constant()?;
                    if let Object(name_obj) = name_val {
                        let Obj::String(name_string) = &*name_obj;
                        let value = self.globals.get(name_string);
                        match value {
                            Some(value) => self.stack_push(value.clone())?,
                            None => {
                                return Err(RuntimeError(format!(
                                    "Undefined variable: {}",
                                    name_string
                                )))
                            }
                        }
                    } else {
                        panic!("Expected string as name, got {:?}", name_val);
                    }
                }
                GetLong => {
                    let name_val = self.read_constant_long()?;
                    if let Object(name_obj) = name_val {
                        let Obj::String(name_string) = &*name_obj;
                        let value = self.globals.get(name_string);
                        match value {
                            Some(value) => self.stack_push(value.clone())?,
                            None => {
                                return Err(RuntimeError(format!(
                                    "Undefined variable: {}",
                                    name_string
                                )))
                            }
                        }
                    } else {
                        panic!("Expected string as name, got {:?}", name_val);
                    }
                }
                DefineGlobal => {
                    let name_val = self.read_constant()?;
                    if let Object(name_obj) = name_val {
                        let value = self.stack_pop()?;
                        let Obj::String(name_string) = &*name_obj;
                        self.globals.set(name_string, value);
                    } else {
                        panic!("Expected string as name, got {:?}", name_val);
                    }
                }
                DefineGlobalLong => {
                    let name_val = self.read_constant_long()?;
                    if let Object(name_obj) = name_val {
                        let value = self.stack_pop()?;
                        let Obj::String(name_string) = &*name_obj;
                        self.globals.set(name_string, value);
                    } else {
                        panic!("Expected string as name, got {:?}", name_val);
                    }
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
                    if let (Number(a_num), Number(b_num)) = (a, b) {
                        self.stack_push(Number(a_num + b_num))?;
                    } else if let (Object(a_obj), Object(b_obj)) = (a, b) {
                        let (Obj::String(a_string), Obj::String(b_string)) = (&*a_obj, &*b_obj);
                        let result = self.gc.alloc_string(format!(
                            "{}{}",
                            a_string.get_value(),
                            b_string.get_value()
                        ));
                        self.stack_push(Value::Object(result))?;
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
                Print => {
                    let value = self.stack_pop()?;
                    vm_print!(self, "{}\n", value);
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
    use crate::assert_eq_str;
    use std::str;

    struct StdoutAdapter<'a> {
        f: &'a mut String,
    }

    impl<'a> io::Write for StdoutAdapter<'a> {
        fn write(&mut self, b: &[u8]) -> Result<usize, io::Error> {
            use std::fmt::Write;
            let s = str::from_utf8(b).map_err(|_| io::Error::from(io::ErrorKind::Other))?;
            print!("{}", s);
            self.f
                .write_str(s)
                .map_err(|_| io::Error::from(io::ErrorKind::Other))?;
            Ok(b.len())
        }

        fn flush(&mut self) -> Result<(), io::Error> {
            Ok(())
        }
    }

    macro_rules! run_chunk_with_gc {
        ($chunk: expr, $gc: expr) => {{
            let mut output = String::new();
            let mut adapter = StdoutAdapter { f: &mut output };
            let mut vm = VM::new(
                VMConfig {
                    trace_instructions: false,
                    trace_stack: false,
                    stdout: &mut adapter,
                },
                &$chunk,
                &mut $gc,
            );
            let result = vm.interpret_chunk(&$chunk);
            (result, output)
        }};
    }

    macro_rules! run_chunk {
        ($chunk:expr) => {{
            let mut gc = GC::new();
            run_chunk_with_gc!($chunk, gc)
        }};
    }

    #[test]
    fn return_wo_constant() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(Return, 1);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Value::Nil));
        assert_eq!(output, "");
    }

    #[test]
    fn constant_wo_return() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(
            result,
            Err(RuntimeError(String::from("Read byte out of bounds")))
        );
    }

    #[test]
    fn constants_break_stack() {
        let mut chunk = Chunk::new();
        for i in 0..257 {
            let const_ref = chunk.add_const(Number(i as f32));
            chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, i);
        }
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Err(RuntimeError(String::from("Stack overflow"))));
    }

    #[test]
    fn return_w_number_constant() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        chunk.write_opcode(Return, 2);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Ok(Number(1.2)));
    }

    #[test]
    fn return_w_bool() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(True, 1);
        chunk.write_opcode(Return, 2);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(true)));
    }

    #[test]
    fn return_w_nil() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::Nil, 1);
        chunk.write_opcode(Return, 2);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Ok(Nil));
    }

    #[test]
    fn return_w_string_literal() {
        let mut gc = GC::new();
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Value::Object(gc.alloc_string("hello world".to_string())));
        chunk.ref_const(const_ref, Constant, ConstantLong, 1);
        chunk.write_opcode(Return, 2);
        let (result, _) = run_chunk_with_gc!(chunk, gc);
        match result {
            Ok(Value::Object(obj)) => assert_eq_str!(obj, "hello world"),
            _ => panic!("Expected object"),
        }
    }

    #[test]
    fn return_w_many_constants() {
        let mut chunk = Chunk::new();
        for i in 0..256 {
            let const_ref = chunk.add_const(Number(i as f32));
            chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, i);
        }
        chunk.write_opcode(Return, 256);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Ok(Number(255.0)));
    }

    #[test]
    fn add_numbers() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Add, 3);
        chunk.write_opcode(Return, 4);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Ok(Number(2.0)));
    }

    #[test]
    fn add_number_nil() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Nil);
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Add, 3);
        chunk.write_opcode(Return, 4);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(
            result,
            Err(RuntimeError(String::from(
                "Invalid type for addition: nil 1"
            )))
        );
    }
    #[test]
    fn add_strings() {
        let mut gc = GC::new();
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Value::Object(gc.alloc_string("hello".to_string())));
        chunk.ref_const(const_ref, Constant, ConstantLong, 1);
        let const_ref = chunk.add_const(Value::Object(gc.alloc_string("world".to_string())));
        chunk.ref_const(const_ref, Constant, ConstantLong, 2);
        chunk.write_opcode(Add, 3);
        chunk.write_opcode(Return, 4);
        println!("Running");
        let (result, _) = run_chunk_with_gc!(chunk, gc);
        match result {
            Ok(Value::Object(obj)) => assert_eq_str!(obj, "helloworld"),
            _ => panic!("Expected object"),
        }
        drop(gc);
    }

    #[test]
    fn subtract_numbers() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(3.4));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Subtract, 3);
        chunk.write_opcode(Return, 4);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Ok(Number(-2.2)));
    }

    #[test]
    fn subtract_number_nil() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Nil);
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Subtract, 3);
        chunk.write_opcode(Return, 4);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(
            result,
            Err(RuntimeError(String::from(
                "Invalid type for subtraction: nil 1"
            )))
        );
    }

    #[test]
    fn multiply_numbers() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(3.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(-0.5));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Multiply, 3);
        chunk.write_opcode(Return, 4);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Ok(Number(-1.5)));
    }

    #[test]
    fn multiply_number_nil() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Nil);
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Multiply, 3);
        chunk.write_opcode(Return, 4);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(
            result,
            Err(RuntimeError(String::from(
                "Invalid type for multiplication: nil 1"
            )))
        );
    }

    #[test]
    fn divide_numbers() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(10.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(2.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Divide, 3);
        chunk.write_opcode(Return, 4);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Ok(Number(5.0)));
    }

    #[test]
    fn divide_number_nil() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Nil);
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Divide, 3);
        chunk.write_opcode(Return, 4);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(
            result,
            Err(RuntimeError(String::from(
                "Invalid type for division: nil 1"
            )))
        );
    }

    #[test]
    fn negate_number() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        chunk.write_opcode(Negate, 2);
        chunk.write_opcode(Return, 3);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Number(-1.2)));
        assert_eq!(output, "");
    }

    #[test]
    fn negate_nil() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Nil);
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        chunk.write_opcode(Negate, 2);
        chunk.write_opcode(Return, 3);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(
            result,
            Err(RuntimeError(String::from("Invalid type for negation: nil")))
        );
    }

    #[test]
    fn not_true() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Boolean(true));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        chunk.write_opcode(Not, 2);
        chunk.write_opcode(Return, 3);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "");
    }

    #[test]
    fn not_nil() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Nil);
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        chunk.write_opcode(Not, 2);
        chunk.write_opcode(Return, 3);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(true)));
        assert_eq!(output, "");
    }

    #[test]
    fn not_zero() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(0.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        chunk.write_opcode(Not, 2);
        chunk.write_opcode(Return, 3);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "");
    }

    #[test]
    fn not_one() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        chunk.write_opcode(Not, 2);
        chunk.write_opcode(Return, 3);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "");
    }

    #[test]
    fn equal_true() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Equal, 3);
        chunk.write_opcode(Return, 4);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(true)));
        assert_eq!(output, "");
    }

    #[test]
    fn equal_false() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(2.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Equal, 3);
        chunk.write_opcode(Return, 4);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "");
    }

    #[test]
    fn greater_true() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(2.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Greater, 3);
        chunk.write_opcode(Return, 4);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(true)));
        assert_eq!(output, "");
    }

    #[test]
    fn greater_false() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Greater, 3);
        chunk.write_opcode(Return, 4);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "");
    }

    #[test]
    fn less_true() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(2.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Less, 3);
        chunk.write_opcode(Return, 4);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(true)));
        assert_eq!(output, "");
    }

    #[test]
    fn less_false() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(1.0));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Less, 3);
        chunk.write_opcode(Return, 4);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Boolean(false)));
        assert_eq!(output, "");
    }

    #[test]
    fn simple_operations() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(3.4));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Add, 1);
        let const_ref = chunk.add_const(Number(5.6));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 3);
        chunk.write_opcode(Divide, 1);
        chunk.write_opcode(Negate, 1);
        chunk.write_opcode(Return, 1);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Ok(Number(-0.82142866)));
    }

    #[test]
    fn simple_print_number() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        chunk.write_opcode(Print, 1);
        chunk.write_opcode(Return, 2);
        let (result, output) = run_chunk!(chunk);
        assert_eq!(result, Ok(Nil));
        assert_eq!(output, "1.2\n");
    }

    #[test]
    fn simple_print_string() {
        let mut gc = GC::new();
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Value::Object(gc.alloc_string("hello world".to_string())));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        chunk.write_opcode(Print, 1);
        chunk.write_opcode(Return, 2);
        let (result, output) = run_chunk_with_gc!(chunk, gc);
        assert_eq!(result, Ok(Nil));
        assert_eq!(output, "\"hello world\"\n");
    }

    #[test]
    fn simple_expression_statement() {
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Number(3.4));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 2);
        chunk.write_opcode(Add, 1);
        chunk.write_opcode(Pop, 2);
        chunk.write_opcode(Return, 3);
        let (result, _) = run_chunk!(chunk);
        assert_eq!(result, Ok(Nil));
    }

    #[test]
    fn global_var_declaration() {
        let mut gc = GC::new();
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Value::Object(gc.alloc_string("x".to_string())));
        chunk.ref_const(const_ref, OpCode::DefineGlobal, OpCode::DefineGlobalLong, 1);
        chunk.write_opcode(Return, 2);
        let (result, _) = run_chunk_with_gc!(chunk, gc);
        assert_eq!(result, Ok(Nil));
    }

    #[test]
    fn global_var_declaration_and_read() {
        let mut gc = GC::new();
        let mut chunk = Chunk::new();
        let const_ref = chunk.add_const(Number(1.2));
        chunk.ref_const(const_ref, OpCode::Constant, OpCode::ConstantLong, 1);
        let const_ref = chunk.add_const(Value::Object(gc.alloc_string("x".to_string())));
        chunk.ref_const(const_ref, OpCode::DefineGlobal, OpCode::DefineGlobalLong, 1);
        let const_ref = chunk.add_const(Value::Object(gc.alloc_string("x".to_string())));
        chunk.ref_const(const_ref, OpCode::Get, OpCode::GetLong, 1);
        chunk.write_opcode(Return, 2);
        let (result, _) = run_chunk_with_gc!(chunk, gc);
        assert_eq!(result, Ok(Number(1.2)));
    }
}
