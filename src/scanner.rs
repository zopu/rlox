use core::panic;
use std::collections::LinkedList;

use crate::tokens::{Token, TokenLiteral, TokenType};

pub struct Scanner {
    source: Vec<char>,
    tokens: LinkedList<Token>,
    start: usize,
    current: usize,
    line: usize,
}

impl Scanner {
    pub fn new(src: &str) -> Self {
        Scanner {
            source: src.chars().collect(),
            tokens: LinkedList::new(),
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_tokens(&mut self) -> &LinkedList<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }

        self.tokens.push_back(Token::new(
            TokenType::Eof,
            "".to_string(),
            TokenLiteral::None,
            self.line,
        ));
        &self.tokens
    }

    fn scan_token(&mut self) {
        let c = self.advance();
        // println!("Scanning char {}", c);
        match c {
            '(' => self.add_token(TokenType::LeftParen),
            ')' => self.add_token(TokenType::RightParen),
            '[' => self.add_token(TokenType::LeftBrace),
            ']' => self.add_token(TokenType::RightBrace),
            ',' => self.add_token(TokenType::Comma),
            '.' => self.add_token(TokenType::Dot),
            '-' => self.add_token(TokenType::Minus),
            '+' => self.add_token(TokenType::Plus),
            ';' => self.add_token(TokenType::SemiColon),
            '*' => self.add_token(TokenType::Star),

            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenType::BangEqual);
                } else {
                    self.add_token(TokenType::Bang);
                }
            }
            '=' => {
                if self.match_char('=') {
                    self.add_token(TokenType::EqualEqual);
                } else {
                    self.add_token(TokenType::Equal);
                }
            }
            '<' => {
                if self.match_char('=') {
                    self.add_token(TokenType::LessEqual);
                } else {
                    self.add_token(TokenType::Less);
                }
            }
            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenType::GreaterEqual);
                } else {
                    self.add_token(TokenType::Greater);
                }
            }
            '/' => {
                if self.match_char('/') {
                    // A comment goes on until the end of the line
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash);
                }
            }

            // Whitespace
            ' ' | '\r' | '\t' => {}
            '\n' => {
                self.line += 1;
            }

            '"' => {
                self.scan_string();
            }

            c if is_digit(c) => {
                self.scan_number();
            }
            _ => {
                panic!("Unexpected token at line {}", self.line);
            }
        }
    }

    fn scan_number(&mut self) {
        while is_digit(self.peek()) {
            self.advance();
        }
        // Look for a fractional/decimal part
        if self.peek() == '.' && is_digit(self.peek_next()) {
            // Consume the '.'
            self.advance();
        }
        while is_digit(self.peek()) {
            self.advance();
        }

        // Parse numbers as f64
        let num_string: String = self.source[self.start..self.current].iter().collect();
        let num: f64 = num_string.parse().unwrap();
        self.add_token_with_literal(TokenType::Number, TokenLiteral::Number(num));
    }

    fn scan_string(&mut self) {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            panic!("Unterminated string on line {}", self.line);
        }

        // Consume the closing "
        self.advance();

        // Trim the surrounding quotes
        let value: String = self.source[self.start + 1..self.current - 1]
            .iter()
            .collect();
        self.add_token_with_literal(TokenType::String, TokenLiteral::String(value));
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn advance(&mut self) -> char {
        let c = self.source[self.current];
        self.current += 1;
        c
    }

    fn add_token(&mut self, t: TokenType) {
        self.add_token_with_literal(t, TokenLiteral::None);
    }

    fn add_token_with_literal(&mut self, t: TokenType, literal: TokenLiteral) {
        let text: String = self.source[self.start..self.current].iter().collect();
        // println!("Adding token {}: {}", t.to_string(), text);
        self.tokens
            .push_back(Token::new(t, text, literal, self.line));
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        if self.source[self.current] != expected {
            return false;
        }
        self.current += 1;
        true
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            return '\0';
        }
        self.source[self.current]
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            return '\0';
        }
        self.source[self.current + 1]
    }
}

fn is_digit(c: char) -> bool {
    c >= '0' && c <= '9'
}
