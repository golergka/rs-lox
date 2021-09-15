mod chunk;
mod compiler;
mod debug;
mod rle;
mod scanner;
mod value;
mod vm;

use crate::chunk::Chunk;
use crate::compiler::compile;
use crate::vm::*;
use std::env;
use std::error::Error;

fn repl() -> Result<(), Box<dyn Error>> {
    loop {
        use std::io::Write;
        use text_io::read;

        let mut stdout = std::io::stdout();
        let empty_chunk = Chunk::new();
        let mut vm = VM::new(
            VMConfig {
                trace_execution: true,
                stdout: &mut stdout,
            },
            &empty_chunk,
        );
        print!("> ");
        std::io::stdout().flush()?;
        let input: String = read!("{}\n");
        match compile(input) {
            Ok(chunk) => match vm.interpret_chunk(&chunk) {
                Ok(result) => println!("{}", result),
                Err(error) => println!("{}", error),
            },
            Err(e) => println!("{}", e),
        };
    }
}

fn run_file(path: &str) -> Result<(), Box<dyn Error>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut contents = String::new();
    let mut stdout = std::io::stdout();
    file.read_to_string(&mut contents)?;
    return match compile(contents) {
        Ok(chunk) => {
            let mut vm = VM::new(
                VMConfig {
                    trace_execution: true,
                    stdout: &mut stdout,
                },
                &chunk,
            );
            match vm.run() {
                Ok(result) => {
                    println!("{}", result);
                    Ok(())
                }
                Err(e) => Err(Box::new(e)),
            }
        }
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
