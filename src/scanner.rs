use std::collections::HashMap;
use std::collections::LinkedList;

use crate::errors::ErrorReporter;
use crate::tokens::{Token, TokenLiteral, TokenType};

pub struct Scanner<'a> {
    source: Vec<char>,
    tokens: LinkedList<Token>,
    start: usize,
    current: usize,
    line: usize,
    kw_map: HashMap<String, TokenType>,
    error_reporter: &'a ErrorReporter,
}

impl<'a> Scanner<'a> {
    pub fn new(src: &str, error_reporter: &'a ErrorReporter) -> Self {
        let mut kw_map: HashMap<String, TokenType> = HashMap::new();
        kw_map.insert("and".to_string(), TokenType::And);
        kw_map.insert("class".to_string(), TokenType::Class);
        kw_map.insert("else".to_string(), TokenType::Else);
        kw_map.insert("false".to_string(), TokenType::False);
        kw_map.insert("for".to_string(), TokenType::For);
        kw_map.insert("fun".to_string(), TokenType::Fun);
        kw_map.insert("if".to_string(), TokenType::If);
        kw_map.insert("nil".to_string(), TokenType::Nil);
        kw_map.insert("or".to_string(), TokenType::Or);
        kw_map.insert("print".to_string(), TokenType::Print);
        kw_map.insert("return".to_string(), TokenType::Return);
        kw_map.insert("super".to_string(), TokenType::Super);
        kw_map.insert("this".to_string(), TokenType::This);
        kw_map.insert("true".to_string(), TokenType::True);
        kw_map.insert("var".to_string(), TokenType::Var);
        kw_map.insert("while".to_string(), TokenType::While);

        Scanner {
            source: src.chars().collect(),
            tokens: LinkedList::new(),
            start: 0,
            current: 0,
            line: 1,
            kw_map,
            error_reporter,
        }
    }

    pub fn scan_tokens(mut self) -> LinkedList<Token> {
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
        self.tokens
    }

    fn scan_token(&mut self) {
        let c = self.advance();
        // println!("Scanning char {}", c);
        match c {
            '(' => self.add_token(TokenType::LeftParen),
            ')' => self.add_token(TokenType::RightParen),
            '[' => self.add_token(TokenType::LeftBrace),
            ']' => self.add_token(TokenType::RightBrace),
            ':' => self.add_token(TokenType::Colon),
            ',' => self.add_token(TokenType::Comma),
            '.' => self.add_token(TokenType::Dot),
            '-' => self.add_token(TokenType::Minus),
            '+' => self.add_token(TokenType::Plus),
            '?' => self.add_token(TokenType::QuestionMark),
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
                } else if self.match_char('*') {
                    // Multi-line comment
                    let start_line = self.line;
                    while !self.is_at_end() && (self.peek() != '*' || self.peek_next() != '/') {
                        if self.peek() == '\n' {
                            self.line += 1;
                        }
                        self.advance();
                    }
                    if self.is_at_end() {
                        self.error_reporter
                            .error(start_line, "Unterminated multi-line comment on line {}");
                    }
                    // Consume the closing */
                    self.advance();
                    self.advance();
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
            c if is_alpha(c) => {
                self.scan_identifier();
            }

            _ => {
                self.error_reporter
                    .error(self.line, "Unexpected token at line {}");
            }
        }
    }

    fn scan_identifier(&mut self) {
        while is_alphanumeric(self.peek()) {
            self.advance();
        }
        let text: String = self.source[self.start..self.current].iter().collect();
        let token_type = self
            .kw_map
            .get(&text)
            .cloned()
            .unwrap_or(TokenType::Identifier);
        self.add_token(token_type);
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
            self.error_reporter
                .error(self.line, "Unterminated string on line {}");
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

fn is_alpha(c: char) -> bool {
    (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_'
}

fn is_alphanumeric(c: char) -> bool {
    is_alpha(c) || is_digit(c)
}
