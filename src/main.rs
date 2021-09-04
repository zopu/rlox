use std::collections::LinkedList;
use std::env;
use std::io;
use std::io::BufRead;
use std::io::Write;

mod scanner;
mod tokens;

use scanner::Scanner;
use tokens::Token;

static mut HAD_ERROR: bool = false;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        println!("Usage: rlox [script]");
        std::process::exit(64);
    } else if args.len() == 2 {
        run_file(&args[1]);
    } else {
        run_prompt();
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
        if let Ok(_) = stdin.lock().read_line(&mut buf) {
            run(&buf);
            unsafe {
                HAD_ERROR = false;
            }
            buf.clear();
        }
    }
}

fn run(code: &str) {
    let mut scanner: Scanner = Scanner::new(code);
    let tokens: &LinkedList<Token> = scanner.scan_tokens();

    for t in tokens {
        println!("Token: {:?}", t)
    }

    unsafe {
        if HAD_ERROR {
            std::process::exit(65)
        }
    }
}

fn error(line: u32, message: &str) {
    report(line, "", message);
}

fn report(line: u32, location: &str, msg: &str) {
    println!("[line {}] Error {}: {}", line, location, msg);
}
