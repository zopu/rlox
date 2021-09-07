use crate::tokens::{Token, TokenLiteral};

#[derive(Debug)]
pub enum Stmt {
    Expression(Expr),
    Print(Expr),
    Var(VarStmt),
}

#[derive(Debug)]
pub enum Expr {
    Binary(BinaryExpr),
    Grouping(Box<Expr>),
    Literal(TokenLiteral),
    Unary(UnaryExpr),
    Variable(Token),
}

#[derive(Debug)]
pub struct VarStmt {
    pub name: Token,
    pub initializer: Box<Expr>,
}

#[derive(Debug)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

#[derive(Debug)]
pub struct UnaryExpr {
    pub operator: Token,
    pub right: Box<Expr>,
}

pub struct PrettyPrinter {}

impl PrettyPrinter {
    pub fn print_stmt(&self, stmt: &Stmt) -> String {
        match stmt {
            Stmt::Expression(e) => self.print_expr(e),
            Stmt::Print(e) => {
                let mut s = "print ".to_string();
                s.push_str(&self.print_expr(e));
                s
            }
            Stmt::Var(vs) => {
                let mut s = "var ".to_string();
                s.push_str(&vs.name.lexeme);
                s.push_str(&self.print_expr(vs.initializer.as_ref()));
                s
            }
        }
    }

    pub fn print_expr(&self, e: &Expr) -> String {
        match e {
            Expr::Binary(e) => self.parenthesize(&e.operator.lexeme, &[&e.left, &e.right]),
            Expr::Grouping(b) => {
                let e = b.as_ref();
                self.parenthesize("group", &[e])
            }
            Expr::Literal(token_literal) => match token_literal {
                TokenLiteral::None => "nil".to_string(),
                TokenLiteral::True => "true".to_string(),
                TokenLiteral::False => "false".to_string(),
                TokenLiteral::Nil => "nil".to_string(),
                TokenLiteral::String(s) => s.clone(),
                TokenLiteral::Number(n) => n.to_string(),
            },
            Expr::Unary(e) => self.parenthesize(&e.operator.lexeme, &[&e.right]),
            Expr::Variable(token) => token.lexeme.clone(),
        }
    }

    fn parenthesize(&self, name: &str, exprs: &[&Expr]) -> String {
        let mut s = "(".to_string();
        s.push_str(name);
        for e in exprs {
            s.push(' ');
            s.push_str(&self.print_expr(e));
        }
        s.push_str(")");
        s
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tokens::{Token, TokenLiteral, TokenType};

    #[test]
    pub fn can_pretty_print() {
        let e = Expr::Binary(BinaryExpr {
            left: Box::new(Expr::Literal(TokenLiteral::Number(1.23))),
            operator: Token {
                token_type: TokenType::Plus,
                lexeme: "+".to_string(),
                literal: TokenLiteral::None,
                line: 1,
            },
            right: Box::new(Expr::Literal(TokenLiteral::Number(4.5))),
        });

        let pp = PrettyPrinter {};
        let s = pp.print_expr(&e);
        println!("AST: {}", s);
    }
}
