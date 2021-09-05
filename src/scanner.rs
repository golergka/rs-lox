use crate::chunk::{LineNumber};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // Single-character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,
    // One or two character tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    // Literals
    Identifier,
    String,
    Number,
    // Keywords
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    
    Error,
    Eof
}

pub struct Token<'a> {
    pub kind: TokenKind,
    pub lexeme: &'a str,
    pub line: LineNumber,
}

impl<'a> Token<'a> {
    pub fn new(kind: TokenKind, lexeme: &'a str, line: LineNumber) -> Token<'a> {
        Token {
            kind,
            lexeme,
            line,
        }
    }
}

pub struct Scanner<'a> {
    input: &'a String,
    start: usize,
    current: usize,
    line: LineNumber
}

impl Scanner<'_> {
    pub fn new(input: &String) -> Scanner {
        Scanner {
            input,
            start: 0,
            current: 0,
            line: 1
        }
    }
    
    fn is_at_end(&self) -> bool {
        self.current >= self.input.len()
    }
    
    fn make_token(&self, kind: TokenKind) -> Token {
        let lexeme = &self.input[self.start..self.current];
        Token::new(kind, lexeme, self.line)
    }
    
    fn error_token<'a>(&self, message: &'a str) -> Token<'a> {
        Token::new(TokenKind::Error, message, self.line)
    }
    
    pub fn scan(&mut self) -> Token {
        self.start = self.current;
        if self.is_at_end() {
            return self.make_token(TokenKind::Eof);
        }

        return self.error_token("Unexpected character");
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn empty_input() {
        let input = String::from("");
        let mut scanner = Scanner::new(&input);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Eof);
    }
}