use crate::tokens::{Token, TokenLiteral};

pub trait VisitableExpr<T> {
    fn accept(&self, v: &mut dyn Visitor<T>) -> T;
}

pub enum Expr {
    BinaryExpr(BinaryExpr),
    GroupingExpr(GroupingExpr),
    LiteralExpr(LiteralExpr),
    UnaryExpr(UnaryExpr),
}

impl<T> VisitableExpr<T> for Expr {
    fn accept(&self, v: &mut dyn Visitor<T>) -> T {
        match self {
            Expr::BinaryExpr(e) => e.accept(v),
            Expr::GroupingExpr(e) => e.accept(v),
            Expr::LiteralExpr(e) => e.accept(v),
            Expr::UnaryExpr(e) => e.accept(v),
        }
    }
}

pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

impl<T> VisitableExpr<T> for BinaryExpr {
    fn accept(&self, v: &mut dyn Visitor<T>) -> T {
        v.visit_binary(self)
    }
}

pub struct GroupingExpr {
    pub expr: Box<Expr>,
}

impl<T> VisitableExpr<T> for GroupingExpr {
    fn accept(&self, v: &mut dyn Visitor<T>) -> T {
        v.visit_grouping(self)
    }
}

pub struct LiteralExpr {
    pub literal: TokenLiteral,
}

impl<T> VisitableExpr<T> for LiteralExpr {
    fn accept(&self, v: &mut dyn Visitor<T>) -> T {
        v.visit_literal(self)
    }
}

pub struct UnaryExpr {
    pub operator: Token,
    pub right: Box<Expr>,
}

impl<T> VisitableExpr<T> for UnaryExpr {
    fn accept(&self, v: &mut dyn Visitor<T>) -> T {
        v.visit_unary(self)
    }
}

pub trait Visitor<T> {
    fn visit_binary(&mut self, b: &BinaryExpr) -> T;
    fn visit_grouping(&mut self, b: &GroupingExpr) -> T;
    fn visit_literal(&mut self, b: &LiteralExpr) -> T;
    fn visit_unary(&mut self, e: &UnaryExpr) -> T;
}

pub struct PrettyPrinter {}

impl PrettyPrinter {
    pub fn print(&mut self, e: &dyn VisitableExpr<String>) -> String {
        e.accept(self)
    }

    fn parenthesize(&mut self, name: &str, exprs: &[&dyn VisitableExpr<String>]) -> String {
        let mut s = "(".to_string();
        s.push_str(name);
        for e in exprs {
            s.push(' ');
            s.push_str(&e.accept(self));
        }
        s.push_str(")");
        s
    }
}

impl Visitor<String> for PrettyPrinter {
    fn visit_binary(&mut self, e: &BinaryExpr) -> String {
        self.parenthesize(&e.operator.lexeme, &[e.left.as_ref(), e.right.as_ref()])
    }

    fn visit_unary(&mut self, e: &UnaryExpr) -> String {
        self.parenthesize(&e.operator.lexeme, &[e.right.as_ref()])
    }

    fn visit_grouping(&mut self, e: &GroupingExpr) -> String {
        self.parenthesize("group", &[e.expr.as_ref()])
    }

    fn visit_literal(&mut self, e: &LiteralExpr) -> String {
        match &e.literal {
            TokenLiteral::None => "nil".to_string(),
            TokenLiteral::String(s) => s.clone(),
            TokenLiteral::Number(n) => n.to_string(),
        }
    }
}
