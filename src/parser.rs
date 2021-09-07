use thiserror::Error;

use crate::{
    errors::ErrorReporter,
    expr::{self, BinaryExpr, Expr, Stmt, UnaryExpr, VarStmt},
    tokens::{Token, TokenLiteral, TokenType},
};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Expect ')' after expression")]
    RightParenMissing,

    #[error("Expect expression")]
    ExpressionExpected,

    #[error("Expect ':' in ternary operator")]
    ColonExpectedInTernary,

    #[error("Expect ';' after statement")]
    SemiColonExpected,

    #[error("Expect n name")]
    VariableNameExpected,
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

    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::<Stmt>::new();
        while !self.is_at_end() {
            if let Ok(s) = self.declaration() {
                statements.push(s);
            }
        }
        statements
    }

    fn declaration(&mut self) -> Result<Stmt, ParseError> {
        let stmt_result = if self.match_any(&[TokenType::Var]) {
            self.var_declaration()
        } else {
            self.statement()
        };
        if stmt_result.is_err() {
            self.synchronize();
        }
        stmt_result
    }

    fn var_declaration(&mut self) -> Result<Stmt, ParseError> {
        let name = self.consume(TokenType::Identifier, ParseError::VariableNameExpected)?;
        let mut initializer = Expr::Literal(TokenLiteral::Nil);
        if self.match_any(&[TokenType::Equal]) {
            initializer = self.expression()?;
        }
        self.consume(TokenType::SemiColon, ParseError::SemiColonExpected)?;
        Ok(expr::Stmt::Var(VarStmt {
            name,
            initializer: Box::new(initializer),
        }))
    }

    fn statement(&mut self) -> Result<Stmt, ParseError> {
        if self.match_any(&[TokenType::Print]) {
            self.print_statement()
        } else {
            self.expression_statement()
        }
    }

    fn print_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.expression_list()?;
        self.consume(TokenType::SemiColon, ParseError::SemiColonExpected)?;
        Ok(Stmt::Print(expr))
    }

    fn expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.expression_list()?;
        self.consume(TokenType::SemiColon, ParseError::SemiColonExpected)?;
        Ok(Stmt::Expression(expr))
    }

    fn expression_list(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.ternary_conditional()?;
        while self.match_any(&[TokenType::Comma]) {
            let operator = self.previous();
            let right = Box::new(self.ternary_conditional()?);
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right,
            });
        }
        Ok(expr)
    }

    fn ternary_conditional(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.expression()?;
        while self.match_any(&[TokenType::QuestionMark]) {
            let operator = self.previous();
            let true_expr = self.expression()?;
            let colon_op = self.consume(TokenType::Colon, ParseError::ColonExpectedInTernary)?;
            let false_expr = self.expression()?;
            let expr_options = Expr::Binary(BinaryExpr {
                left: Box::new(true_expr),
                operator: colon_op,
                right: Box::new(false_expr),
            });
            expr = Expr::Binary(BinaryExpr {
                left: Box::new(expr),
                operator,
                right: Box::new(expr_options),
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

        if self.match_any(&[TokenType::Identifier]) {
            return Ok(Expr::Variable(self.previous()));
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

    fn synchronize(&mut self) {
        self.advance();
        while !self.is_at_end() {
            if let TokenType::SemiColon = self.previous().token_type {
                return;
            }

            match self.peek().token_type {
                TokenType::Class
                | TokenType::For
                | TokenType::Fun
                | TokenType::If
                | TokenType::Print
                | TokenType::Return
                | TokenType::Var
                | TokenType::While => return,
                _ => {}
            }
            self.advance();
        }
    }
}
