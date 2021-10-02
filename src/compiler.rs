use crate::chunk::OpCode::*;
use crate::chunk::*;
use crate::gc::{GCValue, GC};
use crate::scanner::TokenKind::*;
use crate::scanner::*;
use crate::value::Value;
use crate::vm::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, FromPrimitive, Clone, Copy)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

type ParseFn = for<'a> fn(compiler: &mut Compiler<'a>);

struct ParseRule {
    prefix: Option<ParseFn>,
    infix: Option<ParseFn>,
    precedence: Precedence,
}

fn get_rule(token: TokenKind) -> ParseRule {
    return match token {
        LeftParen => ParseRule {
            prefix: Some(grouping),
            infix: None,
            precedence: Precedence::None,
        },
        RightParen => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        LeftBrace => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        RightBrace => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Comma => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Dot => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Minus => ParseRule {
            prefix: Some(unary),
            infix: Some(binary),
            precedence: Precedence::Term,
        },
        Plus => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Term,
        },
        Semicolon => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Slash => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Factor,
        },
        Star => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Factor,
        },
        Bang => ParseRule {
            prefix: Some(unary),
            infix: None,
            precedence: Precedence::None,
        },
        BangEqual => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Equality,
        },
        TokenKind::Equal => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        EqualEqual => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Equality,
        },
        TokenKind::Greater => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Comparison,
        },
        GreaterEqual => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Comparison,
        },
        TokenKind::Less => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Comparison,
        },
        TokenKind::LessEqual => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Comparison,
        },
        Identifier => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Str => ParseRule {
            prefix: Some(string),
            infix: None,
            precedence: Precedence::None,
        },
        Number => ParseRule {
            prefix: Some(number),
            infix: None,
            precedence: Precedence::None,
        },
        And => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Class => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Else => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::False => ParseRule {
            prefix: Some(literal),
            infix: None,
            precedence: Precedence::None,
        },
        For => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Fun => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        If => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Nil => ParseRule {
            prefix: Some(literal),
            infix: None,
            precedence: Precedence::None,
        },
        Or => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Print => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Return => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Super => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        This => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::True => ParseRule {
            prefix: Some(literal),
            infix: None,
            precedence: Precedence::None,
        },
        Var => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        While => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Error => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        Eof => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
    };
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParserError {
    pub message: String,
    pub token: Token,
}

struct Compiler<'a> {
    gc: &'a mut GC,
    scanner: Scanner<'a>,
    current: Token,
    previous: Token,
    errors: Vec<ParserError>,
    panic_mode: bool,
    current_chunk: Chunk,
}

impl<'a> Compiler<'a> {
    fn new(mut scanner: Scanner<'a>, gc: &'a mut GC) -> Compiler<'a> {
        let first = scanner.scan();
        Compiler {
            gc,
            current: first.clone(),
            previous: first,
            scanner,
            panic_mode: false,
            errors: Vec::new(),
            current_chunk: Chunk::new(),
        }
    }
    // Error handling
    fn error_at(&mut self, token: Token, message: String) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;
        self.errors.push(ParserError { token, message })
    }
    fn error_at_current(&mut self, message: String) {
        self.error_at(self.current.clone(), message)
    }
    fn error(&mut self, message: String) {
        self.error_at(self.previous.clone(), message)
    }
    // Parsing
    fn consume(&mut self, kind: TokenKind, message: String) {
        if self.current.kind == kind {
            self.advance()
        } else {
            self.error_at_current(message)
        }
    }
    fn advance(&mut self) {
        self.previous = self.current.clone();

        loop {
            self.current = self.scanner.scan();
            if self.current.kind != Error {
                break;
            }
            self.error_at_current(self.current.lexeme.to_string())
        }
    }
    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let prefix_rule = get_rule(self.previous.kind).prefix;
        match prefix_rule {
            None => {
                self.error_at_current(String::from("Expected expression."));
            }
            Some(rule) => {
                rule(self);
                while precedence as u8 <= get_rule(self.current.kind).precedence as u8 {
                    self.advance();
                    if let Some(infix_rule) = get_rule(self.previous.kind).infix {
                        infix_rule(self);
                    }
                    // TODO report error?
                }
            }
        }
    }
    // Expressions
    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }
    // Emitting
    fn emit_opcode(&mut self, opcode: OpCode) {
        self.current_chunk
            .write_chunk(opcode as u8, self.previous.line)
    }
    fn emit_opcodes(&mut self, opcodes: &[OpCode]) {
        for opcode in opcodes {
            self.emit_opcode(*opcode);
        }
    }
    fn emit_return(&mut self) {
        self.emit_opcode(OpCode::Return)
    }
    fn emit_constant(&mut self, value: Value) {
        self.current_chunk.write_constant(value, self.previous.line)
    }
    fn end(mut self) -> Chunk {
        self.emit_return();
        self.current_chunk
    }
}

fn number(compiler: &mut Compiler<'_>) {
    let value = compiler.previous.lexeme.parse::<f32>().unwrap();
    compiler.emit_constant(Value::Number(value));
}

fn unary<'a>(compiler: &mut Compiler<'a>) {
    let op_kind = compiler.previous.kind;
    compiler.parse_precedence(Precedence::Unary);
    match op_kind {
        Minus => {
            compiler.emit_opcode(Negate);
        }
        Bang => {
            compiler.emit_opcode(Not);
        }
        _ => panic!("Invalid unary token kind: {:?}", op_kind),
    };
}

fn binary<'a>(compiler: &mut Compiler<'a>) {
    let op_kind = compiler.previous.kind;
    let rule = get_rule(op_kind);
    let precedence = FromPrimitive::from_u8((rule.precedence as u8) + 1).unwrap();
    compiler.parse_precedence(precedence);
    match op_kind {
        BangEqual => compiler.emit_opcodes(&[OpCode::Equal, Not]),
        EqualEqual => compiler.emit_opcode(OpCode::Equal),
        TokenKind::Greater => compiler.emit_opcode(OpCode::Greater),
        GreaterEqual => compiler.emit_opcodes(&[OpCode::Less, OpCode::Not]),
        TokenKind::Less => compiler.emit_opcode(OpCode::Less),
        LessEqual => compiler.emit_opcodes(&[OpCode::Greater, OpCode::Not]),
        Plus => compiler.emit_opcode(Add),
        Minus => compiler.emit_opcode(Subtract),
        Star => compiler.emit_opcode(Multiply),
        Slash => compiler.emit_opcode(Divide),
        _ => panic!("Invalid binary token kind: {:?}", op_kind),
    }
}

fn literal<'a>(compiler: &mut Compiler<'a>) {
    match compiler.previous.kind {
        TokenKind::True => compiler.emit_opcode(OpCode::True),
        TokenKind::False => compiler.emit_opcode(OpCode::False),
        TokenKind::Nil => compiler.emit_opcode(OpCode::Nil),
        _ => panic!("Invalid literal token kind: {:?}", compiler.previous.kind),
    }
}

fn grouping<'a>(compiler: &mut Compiler<'a>) {
    compiler.expression();
    compiler.consume(
        TokenKind::RightParen,
        String::from("Expect ')' after expression."),
    );
}

fn string<'a>(compiler: &mut Compiler<'a>) {
    let lexeme = &compiler.previous.lexeme;
    let value = lexeme[1..lexeme.len() - 1].to_string();
    let obj = compiler.gc.alloc_string(value);
    compiler.emit_constant(Value::Object(obj));
}

pub fn compile<'a>(source: &'a String, gc: &mut GC) -> Result<Chunk, InterpreterError> {
    let scanner = Scanner::new(&source);
    let mut compiler = Compiler::new(scanner, gc);
    compiler.expression();
    compiler.consume(TokenKind::Eof, String::from("Expect end of expression."));
    match compiler.errors.len() {
        0 => Ok(compiler.end()),
        _ => Err(InterpreterError::CompileError(compiler.errors)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! test_compile {
        ($program:expr) => {{
            let mut gc = GC::new();
            let source = String::from($program);
            let result = compile(&source, &mut gc);
            println!("Compile result: {:?}", result);
            (result, gc)
        }};
    }
    macro_rules! test_compile_ok {
        ($program:expr) => {{
            let (result, gc) = test_compile!($program);
            assert!(result.is_ok());
            (result.unwrap(), gc)
        }};
    }

    #[test]
    fn can_use_gc_after_compiling() {
        let mut gc = GC::new();
        let _ = compile(&"true".to_string(), &mut gc);
        gc.alloc_string("Hello".to_string());
    }

    #[test]
    fn empty() {
        let (result, _) = test_compile!("");
        assert!(!result.is_ok());
    }
    mod literals {
        use super::*;

        #[test]
        fn number_literal() {
            let (chunk, _) = test_compile_ok!("123");
            assert_eq!(chunk.get_constant(0), Value::Number(123.0));
            let expect_code = [Constant as u8, 0, Return as u8];
            assert_eq!(chunk.get_code(), expect_code);
        }

        #[test]
        fn true_literal() {
            let (chunk, _) = test_compile_ok!("true");
            let expect_code = [True as u8, Return as u8];
            assert_eq!(chunk.get_code(), expect_code);
        }

        #[test]
        fn false_literal() {
            let (chunk, _) = test_compile_ok!("false");
            let expect_code = [False as u8, Return as u8];
            assert_eq!(chunk.get_code(), expect_code);
        }

        #[test]
        fn nil_literal() {
            let (chunk, _) = test_compile_ok!("nil");
            let expect_code = [Nil as u8, Return as u8];
            assert_eq!(chunk.get_code(), expect_code);
        }
        #[test]
        fn string_literal() {
            let (chunk, gc) = test_compile_ok!(r#""hello world""#);
            let expect_code = [Constant as u8, 0, Return as u8];
            assert_eq!(chunk.get_code(), expect_code);
            match chunk.get_constant(0) {
                Value::Object(o) => {
                    assert_eq!(*o, GCValue::String("hello world".to_string()));
                }
                _ => panic!("Expect string object"),
            }
            drop(gc);
        }
    }

    #[test]
    fn negate() {
        let (chunk, _) = test_compile_ok!("-123");
        assert_eq!(chunk.get_constant(0), Value::Number(123.0));
        let expect_code = [Constant as u8, 0, Negate as u8, Return as u8];
        assert_eq!(chunk.get_code(), expect_code);
    }

    #[test]
    fn not() {
        let (chunk, _) = test_compile_ok!("!true");
        let expect_code = [True as u8, Not as u8, Return as u8];
        assert_eq!(chunk.get_code(), expect_code);
    }

    #[test]
    fn equal_equal() {
        let (chunk, _) = test_compile_ok!("123 == 123");
        assert_eq!(chunk.get_constant(0), Value::Number(123.0));
        assert_eq!(chunk.get_constant(1), Value::Number(123.0));
        let expect_code = [
            Constant as u8,
            0,
            Constant as u8,
            1,
            Equal as u8,
            Return as u8,
        ];
        assert_eq!(chunk.get_code(), expect_code);
    }

    #[test]
    fn bang_equal() {
        let (chunk, _) = test_compile_ok!("123 != 123");
        assert_eq!(chunk.get_constant(0), Value::Number(123.0));
        assert_eq!(chunk.get_constant(1), Value::Number(123.0));
        let expect_code = [
            Constant as u8,
            0,
            Constant as u8,
            1,
            Equal as u8,
            Not as u8,
            Return as u8,
        ];
        assert_eq!(chunk.get_code(), expect_code);
    }

    #[test]
    fn greater() {
        let (chunk, _) = test_compile_ok!("123 > 123");
        assert_eq!(chunk.get_constant(0), Value::Number(123.0));
        assert_eq!(chunk.get_constant(1), Value::Number(123.0));
        let expect_code = [
            Constant as u8,
            0,
            Constant as u8,
            1,
            Greater as u8,
            Return as u8,
        ];
        assert_eq!(chunk.get_code(), expect_code);
    }
}
