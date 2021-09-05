use std::collections::LinkedList;
use std::env;
use std::io;
use std::io::BufRead;
use std::io::Write;

mod expr;
mod expr2;
mod scanner;
mod tokens;

use scanner::Scanner;
use tokens::Token;

static mut HAD_ERROR: bool = false;

fn main() {
    let e2 = expr2::BinaryExpr {
        left: Box::new(expr2::Expr::LiteralExpr(expr2::LiteralExpr {
            literal: tokens::TokenLiteral::Number(1.23),
        })),
        operator: tokens::Token {
            token_type: tokens::TokenType::Plus,
            lexeme: "+".to_string(),
            literal: tokens::TokenLiteral::None,
            line: 1,
        },
        right: Box::new(expr2::Expr::LiteralExpr(expr2::LiteralExpr {
            literal: tokens::TokenLiteral::Number(4.5),
        })),
    };

    let mut pp2 = expr2::PrettyPrinter {};
    let s2 = pp2.print(&e2);
    println!("AST: {}", s2);

    let e = expr::Expr::Binary(expr::BinaryExpr {
        left: Box::new(expr::Expr::Literal(tokens::TokenLiteral::Number(1.23))),
        operator: tokens::Token {
            token_type: tokens::TokenType::Plus,
            lexeme: "+".to_string(),
            literal: tokens::TokenLiteral::None,
            line: 1,
        },
        right: Box::new(expr::Expr::Literal(tokens::TokenLiteral::Number(4.5))),
    });

    let pp = expr::PrettyPrinter {};
    let s = pp.print(&e);
    println!("AST: {}", s);

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
