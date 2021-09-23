use crate::tokens::{Token, TokenLiteral};

#[derive(Clone, Debug)]
pub enum Stmt {
    Block(Vec<Stmt>),
    Break,
    Class(ClassStmt),
    Expression(Expr),
    Function(FunctionStmt),
    If(IfStmt),
    Print(Expr),
    Return(ReturnStmt),
    While(WhileStmt),
    Var(VarStmt),
}

#[derive(Clone, Debug)]
pub enum Expr {
    Assign(AssignExpr),
    Binary(BinaryExpr),
    Call(CallExpr),
    Get(GetExpr),
    Grouping(Box<Expr>),
    Literal(TokenLiteral),
    Logical(LogicalExpr),
    Set(SetExpr),
    This(Token),
    Unary(UnaryExpr),
    Variable(Token),
}

#[derive(Clone, Debug)]
pub struct ClassStmt {
    pub name: Token,

    // Superclass will only ever be parsed as an Expr::Variable
    pub superclass: Option<Expr>,
    pub methods: Vec<FunctionStmt>,
}

#[derive(Clone, Debug)]
pub struct FunctionStmt {
    pub name: Token,
    pub params: Vec<Token>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub struct IfStmt {
    pub condition: Box<Expr>,
    pub then_branch: Box<Stmt>,
    pub else_branch: Option<Box<Stmt>>,
}

#[derive(Clone, Debug)]
pub struct ReturnStmt {
    pub keyword: Token,
    pub value: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct WhileStmt {
    pub condition: Box<Expr>,
    pub body: Box<Stmt>,
}

#[derive(Clone, Debug)]
pub struct VarStmt {
    pub name: Token,
    pub initializer: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct AssignExpr {
    pub name: Token,
    pub value: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct BinaryExpr {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub paren: Token, // Closing paren (So we have it's location for errors)
    pub arguments: Vec<Expr>,
}

#[derive(Clone, Debug)]
pub struct GetExpr {
    pub name: Token,
    pub object: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct LogicalExpr {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct SetExpr {
    pub object: Box<Expr>,
    pub name: Token,
    pub value: Box<Expr>,
}

#[derive(Clone, Debug)]
pub struct UnaryExpr {
    pub operator: Token,
    pub right: Box<Expr>,
}

pub struct PrettyPrinter {}

impl PrettyPrinter {
    pub fn print_stmt(&self, stmt: &Stmt) -> String {
        match stmt {
            Stmt::Block(vec) => {
                let mut s = String::new();
                for statement in vec {
                    s.push_str(&self.print_stmt(statement));
                }
                s
            }
            Stmt::Break => "break;".to_string(),
            Stmt::Class(ClassStmt {
                name,
                superclass,
                methods,
            }) => {
                let mut s = "class ".to_string();
                s.push_str(&name.lexeme);
                if let Some(Expr::Variable(token)) = superclass {
                    s.push_str(" < ");
                    s.push_str(&token.lexeme);
                    s.push_str(" ");
                }
                s.push_str(" { ");
                for m in methods {
                    s.push_str(&self.print_function_stmt(m));
                }
                s.push_str(" } ");
                s
            }
            Stmt::Expression(e) => self.print_expr(e),
            Stmt::Function(stmt) => self.print_function_stmt(stmt),
            Stmt::If(e) => {
                let mut s = "if (".to_string();
                s.push_str(&self.print_expr(&e.condition));
                s.push_str(") ");
                s.push_str(&self.print_stmt(&e.then_branch));
                if let Some(else_stmt) = &e.else_branch {
                    s.push_str(&self.print_stmt(else_stmt));
                }
                s.push_str(";");
                s
            }
            Stmt::Print(e) => {
                let mut s = "print ".to_string();
                s.push_str(&self.print_expr(e));
                s.push_str(";");
                s
            }
            Stmt::Return(ReturnStmt { keyword: _, value }) => {
                let mut s = "return ".to_string();
                s.push_str(&self.print_expr(value));
                s.push_str(";");
                s
            }
            Stmt::While(WhileStmt { condition, body }) => {
                let mut s = "while (".to_string();
                s.push_str(&self.print_expr(&condition));
                s.push_str(") ");
                s.push_str(&self.print_stmt(&body));
                s
            }
            Stmt::Var(vs) => {
                let mut s = "var ".to_string();
                s.push_str(&vs.name.lexeme);
                s.push_str(&self.print_expr(vs.initializer.as_ref()));
                s.push_str(";");
                s
            }
        }
    }

    pub fn print_expr(&self, e: &Expr) -> String {
        match e {
            Expr::Assign(e) => {
                let mut s = e.name.lexeme.clone();
                s.push_str(" = ");
                s.push_str(&self.print_expr(&e.value));
                s.push_str(";");
                s
            }
            Expr::Binary(e) => self.parenthesize(&e.operator.lexeme, &[&e.left, &e.right]),
            Expr::Call(CallExpr {
                callee,
                paren: _,
                arguments,
            }) => {
                let mut s = self.print_expr(&callee);
                s.push_str("(");
                for arg in arguments {
                    s.push_str(&self.print_expr(&arg));
                }
                s.push_str(")");
                s
            }
            Expr::Get(GetExpr { name, object }) => {
                let mut s = self.print_expr(&object);
                s.push_str(".");
                s.push_str(&name.lexeme);
                s
            }
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
            Expr::Logical(e) => self.parenthesize(&e.operator.lexeme, &[&e.left, &e.right]),
            Expr::Set(e) => {
                let mut s = self.print_expr(&e.object);
                s.push_str(".");
                s.push_str(&e.name.lexeme);
                s.push_str(" = ");
                s.push_str(&self.print_expr(&e.value));
                s
            }
            Expr::This(_) => "this".to_string(),
            Expr::Unary(e) => self.parenthesize(&e.operator.lexeme, &[&e.right]),
            Expr::Variable(token) => token.lexeme.clone(),
        }
    }

    fn print_function_stmt(&self, FunctionStmt { name, params, body }: &FunctionStmt) -> String {
        let mut s = "fun ".to_string();
        s.push_str(&name.lexeme);
        for p in params {
            s.push_str(&p.lexeme)
        }
        s.push('{');
        for stmt in body {
            s.push_str(&self.print_stmt(stmt));
        }
        s.push('}');
        s
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
