use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq, strum_macros::ToString)]
pub enum TokenType {
    // Single-character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Colon,
    Comma,
    Dot,
    Minus,
    Plus,
    QuestionMark,
    SemiColon,
    Slash,
    Star,

    // One or two character tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Identifiers
    Identifier,
    String,
    Number,

    // Keywords
    And,
    Break,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    Eof,
}

#[derive(Clone, Debug)]
pub enum TokenLiteral {
    None,
    True,
    False,
    Nil,
    String(String),
    Number(f64),
}

#[derive(Clone, Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub literal: TokenLiteral,
    pub line: usize,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: String, literal: TokenLiteral, line: usize) -> Self {
        Token {
            token_type,
            lexeme,
            literal,
            line,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.token_type.to_string())?;
        f.write_str(" ")?;
        f.write_str(&self.lexeme)?;
        f.write_str(" ")?;
        // f.write_str(&self.literal);
        Ok(())
    }
}
