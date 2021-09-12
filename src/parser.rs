use thiserror::Error;

use crate::{errors::ErrorReporter, expr::{self, AssignExpr, BinaryExpr, Expr, IfStmt, LogicalExpr, Stmt, UnaryExpr, VarStmt, WhileStmt}, tokens::{Token, TokenLiteral, TokenType}};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Expect ':' in ternary operator")]
    ColonExpectedInTernary,

    #[error("Expect expression")]
    ExpressionExpected,

    #[error("Expect '(' after if")]
    IfStmtLeftParenExpected,

    #[error("Expect ')' in if statement")]
    IfStmtRightParenExpected,

    #[error("Invalid assignment target")]
    InvalidAssignmentTarget,

    #[error("Expect '}}' at end of block")]
    RightBraceExpected,

    #[error("Expect ')' after expression")]
    RightParenMissing,

    #[error("Expect ';' after statement")]
    SemiColonExpected,

    #[error("Expect n name")]
    VariableNameExpected,
    
    #[error("Expect '(' after while")]
    WhileStmtLeftParenExpected,

    #[error("Expect ')' in while statement")]
    WhileStmtRightParenExpected,
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

    pub fn parse_stmts(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::<Stmt>::new();
        while !self.is_at_end() {
            if let Ok(s) = self.declaration() {
                statements.push(s);
            }
        }
        statements
    }

    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.expression_list()
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
        if self.match_any(&[TokenType::If]) {
            return self.if_statement();
        }
        if self.match_any(&[TokenType::Print]) {
            return self.print_statement();
        }
        if self.match_any(&[TokenType::While]) {
            return self.while_statement();
        }
        if self.match_any(&[TokenType::LeftBrace]) {
            return Ok(Stmt::Block(self.block()?));
        }
        self.expression_statement()
    }

    fn if_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LeftParen, ParseError::IfStmtLeftParenExpected)?;
        let condition = Box::new(self.expression_list()?);
        self.consume(TokenType::RightParen, ParseError::IfStmtRightParenExpected)?;
        let then_branch = Box::new(self.statement()?);
        let mut else_branch: Option<Box<Stmt>> = None;
        if self.match_any(&[TokenType::Else]) {
            else_branch = Some(Box::new(self.statement()?));
        }
        Ok(Stmt::If(IfStmt {
            condition,
            then_branch,
            else_branch,
        }))
    }

    fn print_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr = self.expression_list()?;
        self.consume(TokenType::SemiColon, ParseError::SemiColonExpected)?;
        Ok(Stmt::Print(expr))
    }

    fn while_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LeftParen, ParseError::WhileStmtLeftParenExpected)?;
        let condition = Box::new(self.expression_list()?);
        self.consume(TokenType::RightParen, ParseError::WhileStmtRightParenExpected)?;
        let body = Box::new(self.statement()?);

        Ok(Stmt::While(WhileStmt { condition, body }))
    }

    fn block(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts: Vec<Stmt> = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            stmts.push(self.declaration()?);
        }
        self.consume(TokenType::RightBrace, ParseError::RightBraceExpected)?;
        Ok(stmts)
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
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.or()?;
        if self.match_any(&[TokenType::Equal]) {
            let eq_token = self.previous();
            let val = self.assignment()?;

            if let Expr::Variable(name) = expr {
                return Ok(Expr::Assign(AssignExpr {
                    name,
                    value: Box::new(val),
                }));
            }
            return Err(self.error_at(eq_token, ParseError::InvalidAssignmentTarget));
        }
        Ok(expr)
    }

    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.and()?;
        while self.match_any(&[TokenType::Or]) {
            let operator = self.previous();
            let right = Box::new(self.and()?);
            expr = Expr::Logical(LogicalExpr {
                left: Box::new(expr),
                operator,
                right,
            });
        }
        Ok(expr)
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.equality()?;
        while self.match_any(&[TokenType::And]) {
            let operator = self.previous();
            let right = Box::new(self.equality()?);
            expr = Expr::Logical(LogicalExpr {
                left: Box::new(expr),
                operator,
                right,
            });
        }
        Ok(expr)
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
        self.error_at(self.peek(), error)
    }

    fn error_at(&self, token: Token, error: ParseError) -> ParseError {
        self.error_reporter.token_error(token, &error.to_string());
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
