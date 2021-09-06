use std::collections::LinkedList;
use std::env;
use std::io;
use std::io::BufRead;
use std::io::Write;

mod expr;
mod parser;
mod scanner;
mod tokens;

use scanner::Scanner;
use tokens::Token;

static mut HAD_ERROR: bool = false;

mod errors {
    use crate::tokens::{Token, TokenType};
    use std::cell::RefCell;

    pub struct ErrorReporter {
        had_error: RefCell<bool>,
    }

    impl ErrorReporter {
        pub fn new() -> ErrorReporter {
            ErrorReporter {
                had_error: RefCell::new(false),
            }
        }

        pub fn error(&self, line: usize, message: &str) {
            self.report(line, "", message);
        }

        pub fn token_error(&self, t: Token, msg: &str) {
            if let TokenType::Eof = t.token_type {
                self.report(t.line, " at end", msg);
            } else {
                let mut location: String = " at '".to_string();
                location.push_str(&t.lexeme);
                location.push_str("'");
                self.report(t.line, " at ", msg);
            }
        }

        pub fn report(&self, line: usize, location: &str, msg: &str) {
            self.had_error.replace(true);
            println!("[line {}] Error {}: {}", line, location, msg);
        }

        pub fn had_error(&self) -> bool {
            *self.had_error.borrow()
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        n if n > 2 => {
            println!("Usage: rlox [script]");
            std::process::exit(64);
        }
        2 => {
            run_file(&args[1]);
        }
        _ => {
            run_prompt();
        }
    }
}

fn run_file(filename: &str) {
    println!("running file {:?}", filename);
    let contents = std::fs::read_to_string(filename).expect("Could not read input file");
    run(&contents);
}

fn run_prompt() {
    let stdin = io::stdin();
    let mut buf = String::new();

    loop {
        print!("> ");
        io::stdout().lock().flush().unwrap();
        if stdin.lock().read_line(&mut buf).is_ok() {
            run(&buf);
            unsafe {
                HAD_ERROR = false;
            }
            buf.clear();
        }
    }
}

fn run(code: &str) {
    let scanner: Scanner = Scanner::new(code);
    let tokens: LinkedList<Token> = scanner.scan_tokens();

    for t in &tokens {
        println!("Token: {:?}", t)
    }

    unsafe {
        if HAD_ERROR {
            std::process::exit(65)
        }
    }
    let error_reporter = errors::ErrorReporter::new();

    let mut parser = parser::Parser::new(tokens.into_iter().collect(), &error_reporter);

    let result = parser.parse();
    println!("Parsed: {:?}", result);
}
