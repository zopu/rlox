use std::collections::LinkedList;
use std::io;
use std::io::BufRead;
use std::io::Write;
use std::env;

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
        }
    }
}

fn run(code: &str) {
    let scanner: Scanner = Scanner::new(code);
    let tokens: LinkedList<Token> = scanner.scan_tokens();

    for t in tokens {
        println!("Token: {:?}", t)
    }
}

struct Scanner {
}

impl Scanner {
    pub fn new(_src: &str) -> Self {
        Scanner {}
    }

    pub fn scan_tokens(&self) -> LinkedList<Token> {
        return LinkedList::new();
    }
}

#[derive(Debug)]
struct Token {
}