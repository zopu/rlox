use std::collections::LinkedList;
use std::io;
use std::io::BufRead;
use std::io::Write;

use clap::{App, Arg};

mod env;
mod expr;
mod interpreter;
mod loxvalue;
mod parser;
mod scanner;
mod tokens;

use scanner::Scanner;
use tokens::Token;

use crate::expr::PrettyPrinter;

mod errors {
    use crate::tokens::{Token, TokenType};
    use std::{
        cell::RefCell,
        sync::{Arc, Mutex},
    };

    pub struct ErrorReporter {
        errors_collected: Arc<Mutex<Vec<String>>>,
        had_error: RefCell<bool>,
        had_runtime_error: RefCell<bool>,
    }

    impl ErrorReporter {
        pub fn new() -> ErrorReporter {
            ErrorReporter {
                errors_collected: Arc::new(Mutex::new(Vec::new())),
                had_error: RefCell::new(false),
                had_runtime_error: RefCell::new(false),
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
                self.report(t.line, &location, msg);
            }
        }

        pub fn runtime_error(&self, line: usize, msg: &str) {
            self.had_runtime_error.replace(true);
            self.errors_collected
                .lock()
                .unwrap()
                .push(format!("[Line {}] Runtime Error: {}", line, msg));
        }

        pub fn report(&self, line: usize, location: &str, msg: &str) {
            self.had_error.replace(true);
            self.errors_collected
                .lock()
                .unwrap()
                .push(format!("[line {}] Error {}: {}", line, location, msg));
        }

        pub fn had_error(&self) -> bool {
            *self.had_error.borrow()
        }

        pub fn had_runtime_error(&self) -> bool {
            *self.had_runtime_error.borrow()
        }

        pub fn print_collected_errors(&self) {
            for s in &*self.errors_collected.lock().unwrap() {
                println!("{}", s);
            }
        }

        pub fn reset(&mut self) {
            self.had_error.replace(false);
            self.had_runtime_error.replace(false);
        }
    }
}

fn main() {
    let matches = App::new("rlox")
        .version("0.1")
        .arg(
            Arg::with_name("verbose")
                .short("V")
                .long("verbose")
                .help("Verbose output"),
        )
        .arg(Arg::with_name("FILE"))
        .get_matches();

    let verbose = matches.is_present("verbose");
    if let Some(f) = matches.value_of("FILE") {
        run_file(&f, verbose);
        return;
    }
    run_prompt(verbose);
}

fn run_file(filename: &str, verbose: bool) {
    // println!("running file {:?}", filename);
    let contents = std::fs::read_to_string(filename).expect("Could not read input file");
    let error_reporter = errors::ErrorReporter::new();
    run(&contents, false, verbose, &error_reporter);
    if error_reporter.had_error() {
        std::process::exit(65);
    }
    if error_reporter.had_runtime_error() {
        std::process::exit(70);
    }
}

fn run_prompt(verbose: bool) {
    let stdin = io::stdin();
    let mut buf = String::new();
    let mut error_reporter = errors::ErrorReporter::new();

    loop {
        print!("> ");
        io::stdout().lock().flush().unwrap();
        if stdin.lock().read_line(&mut buf).is_ok() {
            run(&buf, true, verbose, &error_reporter);
            error_reporter.reset();
            buf.clear();
        }
    }
}

fn run(code: &str, allow_exprs: bool, verbose: bool, error_reporter: &errors::ErrorReporter) {
    let scanner: Scanner = Scanner::new(code, error_reporter);
    let tokens: LinkedList<Token> = scanner.scan_tokens();

    if verbose {
        for t in &tokens {
            println!("Token: {:?}", t);
        }
    }

    if error_reporter.had_error() {
        error_reporter.print_collected_errors();
    }

    let mut parser = parser::Parser::new(tokens.clone().into_iter().collect(), &error_reporter);
    let mut interpreter = interpreter::Interpreter::new(error_reporter);

    let stmts = parser.parse_stmts();

    if error_reporter.had_error() {
        if allow_exprs {
            // Try to parse and evaluate a statement instead
            let mut expr_parser =
                parser::Parser::new(tokens.into_iter().collect(), &error_reporter);
            if let Ok(expr) = expr_parser.parse_expr() {
                interpreter.interpret_expr(&expr);
                if error_reporter.had_runtime_error() {
                    error_reporter.print_collected_errors();
                }
            } else {
                error_reporter.print_collected_errors();
            }
            return;
        } else {
            error_reporter.print_collected_errors();
            return;
        }
    }

    if verbose {
        let pp = PrettyPrinter {};
        for stmt in &stmts {
            let s = pp.print_stmt(&stmt);
            println!("Parsed: {:?}", s);
        }
    }

    interpreter.interpret(&stmts);
    if error_reporter.had_runtime_error() {
        error_reporter.print_collected_errors();
    }
}
