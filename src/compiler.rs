use crate::scanner::*;

pub fn compile(source: String) {
    let mut scanner = Scanner::new(&source);
    let mut line: i16 = -1;
    loop {
        let token = scanner.scan();
        if token.line != line {
            print!("{} ", token.line);
            line = token.line;
        } else {
            print!("   | ");
        }
        println!("{:?} '{}.{}'\n", token.kind, token.lexeme, token.line);
        match token.kind {
            TokenKind::Eof => break,
            _ => {}
        }
    }
}
