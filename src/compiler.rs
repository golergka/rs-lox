use crate::chunk::*;
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
        TokenKind::LeftParen => ParseRule {
            prefix: Some(grouping),
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::RightParen => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::LeftBrace => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::RightBrace => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Comma => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Dot => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Minus => ParseRule {
            prefix: Some(unary),
            infix: Some(binary),
            precedence: Precedence::Term,
        },
        TokenKind::Plus => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Term,
        },
        TokenKind::Semicolon => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Slash => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Factor,
        },
        TokenKind::Star => ParseRule {
            prefix: None,
            infix: Some(binary),
            precedence: Precedence::Factor,
        },
        TokenKind::Bang => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::BangEqual => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Equal => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::EqualEqual => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Greater => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::GreaterEqual => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Less => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::LessEqual => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Identifier => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::String => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Number => ParseRule {
            prefix: Some(number),
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::And => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Class => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Else => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::False => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::For => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Fun => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::If => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Nil => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Or => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Print => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Return => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Super => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::This => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::True => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Var => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::While => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Error => ParseRule {
            prefix: None,
            infix: None,
            precedence: Precedence::None,
        },
        TokenKind::Eof => ParseRule {
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
    scanner: Scanner<'a>,
    current: Token,
    previous: Token,
    errors: Vec<ParserError>,
    panic_mode: bool,
    current_chunk: Chunk,
}

impl<'a> Compiler<'a> {
    fn new(mut scanner: Scanner<'a>) -> Compiler<'a> {
        let first = scanner.scan();
        Compiler {
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
            if self.current.kind != TokenKind::Error {
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
    fn emit_byte(&mut self, byte: u8) {
        self.current_chunk.write_chunk(byte, self.previous.line)
    }
    fn emit_opcode(&mut self, opcode: OpCode) {
        self.emit_byte(opcode as u8);
    }
    fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
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
        TokenKind::Minus => {
            compiler.emit_opcode(OpCode::Negate);
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
        TokenKind::Plus => compiler.emit_opcode(OpCode::Add),
        TokenKind::Minus => compiler.emit_opcode(OpCode::Subtract),
        TokenKind::Star => compiler.emit_opcode(OpCode::Multiply),
        TokenKind::Slash => compiler.emit_opcode(OpCode::Divide),
        _ => panic!("Invalid binary token kind: {:?}", op_kind),
    }
}

fn grouping<'a>(compiler: &mut Compiler<'a>) {
    compiler.expression();
    compiler.consume(
        TokenKind::RightParen,
        String::from("Expect ')' after expression."),
    );
}

pub fn compile<'a>(source: &'a String) -> Result<Chunk, InterpreterError> {
    let scanner = Scanner::new(&source);
    let mut compiler = Compiler::new(scanner);
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

    #[test]
    fn empty() {
        let source = "".to_string();
        let result = compile(&source);
        println!("Compile result: {:?}", result);
        assert!(!result.is_ok());
    }

    #[test]
    fn number_literal() {
        let source = "123".to_string();
        let result = compile(&source);
        println!("Compile result: {:?}", result);
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.get_constant(0), Value::Number(123.0));
        let expect_code = [OpCode::Constant as u8, 0, OpCode::Return as u8];
        assert_eq!(chunk.get_code(), expect_code);
    }

    #[test]
    fn unary_minus() {
        let source = "-123".to_string();
        let result = compile(&source);
        println!("Compile result: {:?}", result);
        assert!(result.is_ok());
        let chunk = result.unwrap();
        assert_eq!(chunk.get_constant(0), Value::Number(123.0));
        let expect_code = [
            OpCode::Constant as u8,
            0,
            OpCode::Negate as u8,
            OpCode::Return as u8,
        ];
        assert_eq!(chunk.get_code(), expect_code);
    }
}
