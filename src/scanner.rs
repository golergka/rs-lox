use crate::chunk::LineNumber;

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
    // Special tokens
    Error,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub line: LineNumber,
}

impl Token {
    pub fn new(kind: TokenKind, lexeme: &str, line: LineNumber) -> Token {
        Token {
            kind,
            lexeme: String::from(lexeme),
            line,
        }
    }
}

pub struct Scanner<'a> {
    input: &'a String,
    start: usize,
    current: usize,
    line: LineNumber,
}

fn is_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}

fn is_alpha(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
}

impl<'a> Scanner<'a> {
    pub fn new(input: &'a String) -> Scanner {
        Scanner {
            input,
            start: 0,
            current: 0,
            line: 1,
        }
    }
    fn make_token(&self, kind: TokenKind) -> Token {
        let lexeme = &self.input[self.start..self.current];
        Token::new(kind, lexeme, self.line)
    }
    fn error_token(&self, message: &'a str) -> Token {
        Token::new(TokenKind::Error, message, self.line)
    }
    fn peek(&self) -> Option<char> {
        self.input.chars().nth(self.current)
    }
    fn peek_next(&self) -> Option<char> {
        self.input.chars().nth(self.current + 1)
    }
    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.current += 1;
        return Some(c);
    }
    fn r#match(&mut self, expected: char) -> bool {
        let c = self.peek();
        if c == Some(expected) {
            self.advance();
            return true;
        } else {
            return false;
        }
    }
    fn match_while(&mut self, filter: fn(char) -> bool) {
        while let Some(c) = self.peek() {
            if filter(c) {
                self.advance();
            } else {
                break;
            }
        }
    }
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                // Regular whitespace
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                // Line break
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                // Comments
                '/' => {
                    if Some('/') == self.peek_next() {
                        while let Some(c) = self.peek() {
                            match c {
                                '\n' => {
                                    self.line += 1;
                                    self.advance();
                                    break;
                                }
                                _ => {
                                    self.advance();
                                }
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }
    fn string(&mut self) -> Token {
        while let Some(c) = self.peek() {
            match c {
                '"' => {
                    self.current += 1;
                    return self.make_token(TokenKind::String);
                }
                '\n' => {
                    self.line += 1;
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
        self.error_token("Unterminated string.")
    }
    fn number(&mut self) -> Token {
        self.match_while(is_digit);
        if let Some(c) = self.peek() {
            if c == '.' {
                if let Some(c) = self.peek_next() {
                    if is_digit(c) {
                        // Consume the '.'
                        self.advance();
                        self.match_while(is_digit);
                    }
                }
            }
        }
        self.make_token(TokenKind::Number)
    }
    fn check_keyword(&self, start: usize, keyword: &str, kind: TokenKind) -> TokenKind {
        if &self.input[self.start + start..self.current] == keyword {
            kind
        } else {
            TokenKind::Identifier
        }
    }
    fn identifier_type(&self) -> TokenKind {
        match self.input.chars().nth(self.start).unwrap() {
            'a' => return self.check_keyword(1, "nd", TokenKind::And),
            'c' => return self.check_keyword(1, "lass", TokenKind::Class),
            'e' => return self.check_keyword(1, "lse", TokenKind::Else),
            'f' => {
                if self.current - self.start > 1 {
                    match self.input.chars().nth(self.start + 1).unwrap() {
                        'a' => return self.check_keyword(2, "lse", TokenKind::False),
                        'o' => return self.check_keyword(2, "r", TokenKind::For),
                        'u' => return self.check_keyword(2, "n", TokenKind::Fun),
                        _ => {}
                    }
                }
            }
            'i' => return self.check_keyword(1, "f", TokenKind::If),
            'n' => return self.check_keyword(1, "il", TokenKind::Nil),
            'o' => return self.check_keyword(1, "r", TokenKind::Or),
            'p' => return self.check_keyword(1, "rint", TokenKind::Print),
            'r' => return self.check_keyword(1, "eturn", TokenKind::Return),
            't' => {
                if self.current - self.start > 1 {
                    match self.input.chars().nth(self.start + 1).unwrap() {
                        'h' => return self.check_keyword(2, "is", TokenKind::This),
                        'r' => return self.check_keyword(2, "ue", TokenKind::True),
                        _ => {}
                    }
                }
            }
            's' => return self.check_keyword(1, "uper", TokenKind::Super),
            'v' => return self.check_keyword(1, "ar", TokenKind::Var),
            'w' => return self.check_keyword(1, "hile", TokenKind::While),
            _ => {}
        };
        return TokenKind::Identifier;
    }
    fn identifier(&mut self) -> Token {
        self.match_while(|c| is_alpha(c) || is_digit(c));
        self.make_token(self.identifier_type())
    }
    pub fn scan(&mut self) -> Token {
        self.skip_whitespace();
        self.start = self.current;
        let next = self.advance();
        match next {
            None => self.make_token(TokenKind::Eof),
            Some(c) => match c {
                // Single-character tokens
                '(' => self.make_token(TokenKind::LeftParen),
                ')' => self.make_token(TokenKind::RightParen),
                '{' => self.make_token(TokenKind::LeftBrace),
                '}' => self.make_token(TokenKind::RightBrace),
                ';' => self.make_token(TokenKind::Semicolon),
                ',' => self.make_token(TokenKind::Comma),
                '.' => self.make_token(TokenKind::Dot),
                '-' => self.make_token(TokenKind::Minus),
                '+' => self.make_token(TokenKind::Plus),
                '/' => self.make_token(TokenKind::Slash),
                '*' => self.make_token(TokenKind::Star),
                // One or two character tokens
                '!' => {
                    if self.r#match('=') {
                        self.make_token(TokenKind::BangEqual)
                    } else {
                        self.make_token(TokenKind::Bang)
                    }
                }
                '=' => {
                    if self.r#match('=') {
                        self.make_token(TokenKind::EqualEqual)
                    } else {
                        self.make_token(TokenKind::Equal)
                    }
                }
                '>' => {
                    if self.r#match('=') {
                        self.make_token(TokenKind::GreaterEqual)
                    } else {
                        self.make_token(TokenKind::Greater)
                    }
                }
                '<' => {
                    if self.r#match('=') {
                        self.make_token(TokenKind::LessEqual)
                    } else {
                        self.make_token(TokenKind::Less)
                    }
                }
                // Literals
                '"' => self.string(),
                _ if is_digit(c) => self.number(),
                _ if is_alpha(c) => self.identifier(),
                _ => self.error_token("Unexpected character."),
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    mod single_char {
        use super::*;

        #[test]
        fn left_paren() {
            let input = String::from("(");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::LeftParen);
            assert_eq!(result.lexeme, "(");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn right_paren() {
            let input = String::from(")");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::RightParen);
            assert_eq!(result.lexeme, ")");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn left_brace() {
            let input = String::from("{");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::LeftBrace);
            assert_eq!(result.lexeme, "{");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn right_brace() {
            let input = String::from("}");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::RightBrace);
            assert_eq!(result.lexeme, "}");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn semicolon() {
            let input = String::from(";");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Semicolon);
            assert_eq!(result.lexeme, ";");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn comma() {
            let input = String::from(",");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Comma);
            assert_eq!(result.lexeme, ",");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn dot() {
            let input = String::from(".");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Dot);
            assert_eq!(result.lexeme, ".");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn minus() {
            let input = String::from("-");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Minus);
            assert_eq!(result.lexeme, "-");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn plus() {
            let input = String::from("+");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Plus);
            assert_eq!(result.lexeme, "+");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn slash() {
            let input = String::from("/");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Slash);
            assert_eq!(result.lexeme, "/");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn star() {
            let input = String::from("*");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Star);
            assert_eq!(result.lexeme, "*");
            assert_eq!(result.line, 1);
        }
    }
    mod one_or_two_chars {
        use super::*;
        #[test]
        fn bang() {
            let input = String::from("!");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Bang);
            assert_eq!(result.lexeme, "!");
            assert_eq!(result.line, 1);
        }
        #[test]
        fn bang_equal() {
            let input = String::from("!=");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::BangEqual);
            assert_eq!(result.lexeme, "!=");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn equal() {
            let input = String::from("=");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Equal);
            assert_eq!(result.lexeme, "=");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn equal_equal() {
            let input = String::from("==");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::EqualEqual);
            assert_eq!(result.lexeme, "==");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn greater() {
            let input = String::from(">");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Greater);
            assert_eq!(result.lexeme, ">");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn greater_equal() {
            let input = String::from(">=");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::GreaterEqual);
            assert_eq!(result.lexeme, ">=");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn less() {
            let input = String::from("<");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Less);
            assert_eq!(result.lexeme, "<");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn less_equal() {
            let input = String::from("<=");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::LessEqual);
            assert_eq!(result.lexeme, "<=");
            assert_eq!(result.line, 1);
        }
    }
    mod literals {
        use super::*;

        #[test]
        fn identifier() {
            let input = String::from("foobar");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Identifier);
            assert_eq!(result.lexeme, "foobar");
            assert_eq!(result.line, 1);
        }
        #[test]
        fn identifier_with_underscore() {
            let input = String::from("foo_bar");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Identifier);
            assert_eq!(result.lexeme, "foo_bar");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn identifier_with_digit() {
            let input = String::from("foo1");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Identifier);
            assert_eq!(result.lexeme, "foo1");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn string() {
            let input = String::from("\"foobar\"");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::String);
            assert_eq!(result.lexeme, "\"foobar\"");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn unterminated_string() {
            let input = String::from("\"This is a string");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Error);
            assert_eq!(result.lexeme, "Unterminated string.");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn number() {
            let input = String::from("123");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Number);
            assert_eq!(result.lexeme, "123");
            assert_eq!(result.line, 1);
        }
        #[test]
        fn number_with_decimal() {
            let input = String::from("123.456");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Number);
            assert_eq!(result.lexeme, "123.456");
            assert_eq!(result.line, 1);
        }
    }
    mod keywords {
        use super::*;

        #[test]
        fn and() {
            let input = String::from("and");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::And);
            assert_eq!(result.lexeme, "and");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn class() {
            let input = String::from("class");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Class);
            assert_eq!(result.lexeme, "class");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn r#else() {
            let input = String::from("else");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Else);
            assert_eq!(result.lexeme, "else");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn r#false() {
            let input = String::from("false");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::False);
            assert_eq!(result.lexeme, "false");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn r#for() {
            let input = String::from("for");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::For);
            assert_eq!(result.lexeme, "for");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn fun() {
            let input = String::from("fun");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Fun);
            assert_eq!(result.lexeme, "fun");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn r#if() {
            let input = String::from("if");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::If);
            assert_eq!(result.lexeme, "if");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn nil() {
            let input = String::from("nil");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Nil);
            assert_eq!(result.lexeme, "nil");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn or() {
            let input = String::from("or");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Or);
            assert_eq!(result.lexeme, "or");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn print() {
            let input = String::from("print");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Print);
            assert_eq!(result.lexeme, "print");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn r#return() {
            let input = String::from("return");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Return);
            assert_eq!(result.lexeme, "return");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn super_() {
            let input = String::from("super");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Super);
            assert_eq!(result.lexeme, "super");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn this() {
            let input = String::from("this");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::This);
            assert_eq!(result.lexeme, "this");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn true_() {
            let input = String::from("true");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::True);
            assert_eq!(result.lexeme, "true");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn var() {
            let input = String::from("var");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Var);
            assert_eq!(result.lexeme, "var");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn r#while() {
            let input = String::from("while");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::While);
            assert_eq!(result.lexeme, "while");
            assert_eq!(result.line, 1);
        }
    }
    mod special {
        use super::*;
        #[test]
        fn error() {
            let input = String::from("@");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Error);
            assert_eq!(result.lexeme, "Unexpected character.");
            assert_eq!(result.line, 1);
        }

        #[test]
        fn empty_input() {
            let input = String::from("");
            let mut scanner = Scanner::new(&input);
            let result = scanner.scan();
            assert_eq!(result.kind, TokenKind::Eof);
        }
    }

    #[test]
    fn skips_whitespace() {
        let input = String::from(" ( ) ");
        let mut scanner = Scanner::new(&input);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::LeftParen);
        assert_eq!(result.lexeme, "(");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::RightParen);
        assert_eq!(result.lexeme, ")");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Eof);
        assert_eq!(result.lexeme, "");
        assert_eq!(result.line, 1);
    }

    #[test]
    fn skips_comments() {
        let input = String::from("// This is a comment\n// This is another comment\n");
        let mut scanner = Scanner::new(&input);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Eof);
        assert_eq!(result.lexeme, "");
        assert_eq!(result.line, 3);
    }

    #[test]
    fn scans_sequence() {
        let input = String::from("var five = 5;");
        let mut scanner = Scanner::new(&input);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Var);
        assert_eq!(result.lexeme, "var");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Identifier);
        assert_eq!(result.lexeme, "five");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Equal);
        assert_eq!(result.lexeme, "=");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Number);
        assert_eq!(result.lexeme, "5");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Semicolon);
        assert_eq!(result.lexeme, ";");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Eof);
        assert_eq!(result.lexeme, "");
        assert_eq!(result.line, 1);
    }

    #[test]
    fn advances_lines() {
        let input = String::from("var five = 5;\nvar ten = 10;");
        let mut scanner = Scanner::new(&input);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Var);
        assert_eq!(result.lexeme, "var");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Identifier);
        assert_eq!(result.lexeme, "five");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Equal);
        assert_eq!(result.lexeme, "=");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Number);
        assert_eq!(result.lexeme, "5");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Semicolon);
        assert_eq!(result.lexeme, ";");
        assert_eq!(result.line, 1);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Var);
        assert_eq!(result.lexeme, "var");
        assert_eq!(result.line, 2);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Identifier);
        assert_eq!(result.lexeme, "ten");
        assert_eq!(result.line, 2);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Equal);
        assert_eq!(result.lexeme, "=");
        assert_eq!(result.line, 2);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Number);
        assert_eq!(result.lexeme, "10");
        assert_eq!(result.line, 2);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Semicolon);
        assert_eq!(result.lexeme, ";");
        assert_eq!(result.line, 2);
        let result = scanner.scan();
        assert_eq!(result.kind, TokenKind::Eof);
        assert_eq!(result.lexeme, "");
        assert_eq!(result.line, 2);
    }
}
