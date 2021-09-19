use std::{borrow::Borrow, collections::HashMap};

use crate::{
    ast::{
        AssignExpr, BinaryExpr, CallExpr, Expr, FunctionStmt, IfStmt, LogicalExpr, ReturnStmt,
        Stmt, UnaryExpr, VarStmt, WhileStmt,
    },
    errors::ErrorReporter,
    interpreter::Interpreter,
    tokens::{Token, TokenLiteral},
};

#[derive(Clone)]
enum FunctionType {
    None,
    Function,
}

pub struct Resolver<'a, 'b, 'c> {
    interpreter: &'b mut Interpreter<'a, 'c>,
    error_reporter: &'a ErrorReporter,
    scopes_stack: Vec<HashMap<String, bool>>,
    current_function: FunctionType,
}

impl<'a, 'b, 'c> Resolver<'a, 'b, 'c> {
    pub fn new(
        interpreter: &'b mut Interpreter<'a, 'c>,
        error_reporter: &'a ErrorReporter,
    ) -> Resolver<'a, 'b, 'c> {
        Resolver {
            interpreter,
            error_reporter,
            scopes_stack: Vec::new(),
            current_function: FunctionType::None,
        }
    }

    // resolve_stmts and resolve_expr are wrappers around "inner" private functions here
    // that don't consume self and release the interpreter mut ref. The intention is
    // that users of Resolver are free to use the interpreter after resolution, but in
    // recursive calls we don't want to release it.

    pub fn resolve_stmts(mut self, stmts: &[Stmt]) {
        self.resolve_stmts_inner(stmts);
    }

    pub fn resolve_expr(mut self, expr: &Expr) {
        self.resolve_expr_inner(expr);
    }

    fn resolve_stmts_inner(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            self.resolve_stmt(s);
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block(stmts) => {
                self.begin_scope();
                self.resolve_stmts_inner(stmts);
                self.end_scope();
            }
            Stmt::Function(stmt) => {
                self.declare(&stmt.name.lexeme);
                self.define(&stmt.name.lexeme);
                self.resolve_function(stmt, FunctionType::Function);
            }
            Stmt::Var(VarStmt { name, initializer }) => {
                self.declare(&name.lexeme);
                // Not sure whether we should care about the distinction b/w
                // var a;
                // and
                // var a = nil;
                // which are both currently represented identically in the AST.
                match initializer.borrow() {
                    Expr::Literal(TokenLiteral::Nil) => {}
                    expr => {
                        self.resolve_expr_inner(expr);
                    }
                }
                self.define(&name.lexeme);
            }
            Stmt::If(IfStmt {
                condition,
                then_branch,
                else_branch,
            }) => {
                self.resolve_expr_inner(condition.borrow());
                self.resolve_stmt(then_branch.borrow());
                if let Some(else_branch) = else_branch {
                    self.resolve_stmt(else_branch.borrow());
                }
            }
            Stmt::Print(expr) => self.resolve_expr_inner(expr),
            Stmt::Return(ReturnStmt { keyword: _, value }) => {
                if let FunctionType::None = self.current_function {
                    self.error_reporter
                        .runtime_error(0, "Can't return from top-level code");
                }
                self.resolve_expr_inner(value.borrow());
            }
            Stmt::While(WhileStmt { condition, body }) => {
                self.resolve_expr_inner(condition.borrow());
                self.resolve_stmt(body.borrow());
            }
            Stmt::Break => {}
            Stmt::Expression(expr) => self.resolve_expr_inner(expr),
        }
    }

    fn resolve_expr_inner(&mut self, expr: &Expr) {
        match expr {
            Expr::Assign(AssignExpr { name, value }) => {
                self.resolve_expr_inner(value.borrow());
                self.resolve_local(expr, name);
            }
            Expr::Variable(token) => {
                if let Some(scope) = self.scopes_stack.last() {
                    if let Some(false) = scope.get(&token.lexeme) {
                        // TODO
                        // Return an error here
                        panic!("Variable is undefined");
                    }
                }
                self.resolve_local(expr, token);
            }
            Expr::Binary(BinaryExpr {
                left,
                operator: _,
                right,
            }) => {
                self.resolve_expr_inner(left.borrow());
                self.resolve_expr_inner(right.borrow());
            }
            Expr::Call(CallExpr {
                callee,
                paren: _,
                arguments,
            }) => {
                self.resolve_expr_inner(callee.borrow());
                for arg in arguments {
                    self.resolve_expr_inner(arg);
                }
            }
            Expr::Grouping(expr) => self.resolve_expr_inner(expr.borrow()),
            Expr::Literal(_) => {}
            Expr::Logical(LogicalExpr {
                left,
                operator: _,
                right,
            }) => {
                self.resolve_expr_inner(left.borrow());
                self.resolve_expr_inner(right.borrow());
            }
            Expr::Unary(UnaryExpr { operator: _, right }) => {
                self.resolve_expr_inner(right.borrow());
            }
        }
    }

    fn resolve_local(&mut self, expr: &Expr, name: &Token) {
        for (i, scope) in self.scopes_stack.iter().rev().enumerate() {
            if scope.contains_key(&name.lexeme) {
                // println!("Resolving {} which has ptr {:?} and distance {}", name.lexeme, expr as *const Expr, i);
                self.interpreter.resolve(expr, i);
                return;
            }
        }
    }

    fn resolve_function(&mut self, stmt: &FunctionStmt, ftype: FunctionType) {
        let enclosing_function = self.current_function.clone();
        self.current_function = ftype;
        self.begin_scope();
        for token in &stmt.params {
            self.declare(&token.lexeme);
            self.define(&token.lexeme);
        }
        self.resolve_stmts_inner(&stmt.body);
        self.end_scope();
        self.current_function = enclosing_function;
    }

    fn begin_scope(&mut self) {
        self.scopes_stack.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes_stack.pop();
    }

    fn declare(&mut self, name: &str) {
        match self.scopes_stack.last_mut() {
            None => {}
            Some(scope) => {
                if scope.contains_key(&name.to_string()) {
                    self.error_reporter.runtime_error(
                        0,
                        &format!(
                            "Already a varibale with this name in this scope: '{}'",
                            name
                        ),
                    );
                }
                scope.insert(name.to_string(), false);
            }
        }
    }

    fn define(&mut self, name: &str) {
        match self.scopes_stack.last_mut() {
            None => {}
            Some(scope) => {
                scope.insert(name.to_string(), true);
            }
        }
    }
}
