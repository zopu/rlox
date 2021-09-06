use thiserror::Error;

use crate::{
    errors::ErrorReporter,
    expr::{BinaryExpr, Expr, UnaryExpr},
    tokens::{Token, TokenLiteral, TokenType},
};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Expect ')' after expression")]
    RightParenMissing,

    #[error("Expect expression")]
    ExpressionExpected,
}

pub struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    error_reporter: &'a ErrorReporter,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, error_reporter: &'a ErrorReporter) -> Self {
        Parser {
            tokens,
            current: 0,
            error_reporter,
        }
    }

    pub fn parse(&mut self) -> Option<Expr> {
        match self.expression_list() {
            Ok(expr) => Some(expr),
            Err(_) => None,
        }
    }

    fn expression_list(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.expression()?;
        while self.match_any(&[TokenType::Comma]) {
            let operator = self.previous();
            let right = Box::new(self.expression_list()?);
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
            });
        }
        Ok(expr)
    }

    fn expression(&mut self) -> Result<Expr, ParseError> {
        self.equality()
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.comparison()?;
        while self.match_any(&[TokenType::BangEqual, TokenType::EqualEqual]) {
            let operator = self.previous();
            let right = Box::new(self.comparison()?);
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
            });
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.term()?;
        while self.match_any(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let operator = self.previous();
            let right = Box::new(self.term()?);
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
            });
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.factor()?;
        while self.match_any(&[TokenType::Minus, TokenType::Plus]) {
            let operator = self.previous();
            let right = Box::new(self.factor()?);
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
            });
        }
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.unary()?;
        while self.match_any(&[TokenType::Slash, TokenType::Star]) {
            let operator = self.previous();
            let right = Box::new(self.unary()?);
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
            });
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_any(&[TokenType::Bang, TokenType::Minus]) {
            Ok(Expr::Unary(UnaryExpr {
                operator: self.previous(),
                right: Box::new(self.unary()?),
            }))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        if self.match_any(&[TokenType::False]) {
            return Ok(Expr::Literal(TokenLiteral::False));
        }
        if self.match_any(&[TokenType::True]) {
            return Ok(Expr::Literal(TokenLiteral::True));
        }
        if self.match_any(&[TokenType::Nil]) {
            return Ok(Expr::Literal(TokenLiteral::Nil));
        }

        if self.match_any(&[TokenType::Number, TokenType::String]) {
            return Ok(Expr::Literal(self.previous().literal));
        }

        if self.match_any(&[TokenType::LeftParen]) {
            let expr = self.expression()?;
            self.consume(TokenType::RightParen, ParseError::RightParenMissing)?;
            return Ok(Expr::Grouping(Box::new(expr)));
        }

        Err(self.error(ParseError::ExpressionExpected))
    }

    fn consume(&mut self, tt: TokenType, error: ParseError) -> Result<Token, ParseError> {
        if self.check(&tt) {
            return Ok(self.advance());
        }
        Err(self.error(error))
    }

    fn match_any(&mut self, token_types: &[TokenType]) -> bool {
        for tt in token_types {
            if self.check(tt) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&self, tt: &TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        *tt == self.peek().token_type
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        if let TokenType::Eof = self.peek().token_type {
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Token {
        self.tokens[self.current].clone()
    }

    fn previous(&self) -> Token {
        self.tokens[self.current - 1].clone()
    }

    fn error(&self, error: ParseError) -> ParseError {
        self.error_reporter
            .token_error(self.peek(), &error.to_string());
        error
    }
}
