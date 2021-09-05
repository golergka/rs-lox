mod chunk;
mod compiler;
mod debug;
mod rle;
mod value;
mod vm;
mod scanner;

use crate::vm::interpret;
use std::env;
use std::error::Error;

fn repl() -> Result<(), Box<dyn Error>> {
    loop {
        use std::io::Write;
        use text_io::read;

        print!("> ");
        std::io::stdout().flush()?;
        let input: String = read!("{}\n");
        match interpret(input) {
            Ok(result) => println!("{}", result),
            Err(error) => println!("{}", error),
        };
    }
}

fn run_file(path: &str) -> Result<(), Box<dyn Error>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    return match interpret(contents) {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e)),
    };
}

fn main() -> Result<(), Box<dyn Error>> {
    let argv: Vec<String> = env::args().collect();
    return match argv.len() {
        1 => repl(),
        2 => run_file(&argv[1]),
        _ => {
            use std::io::ErrorKind;
            println!("Usage: rlox [script]");
            Err(Box::new(std::io::Error::new(
                ErrorKind::InvalidInput,
                "invalid command line input",
            )) as Box<dyn Error>)
        }
    };
}
