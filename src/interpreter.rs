use std::{cell::RefCell, convert::TryFrom, rc::Rc, sync::Arc, time::SystemTime};
use thiserror::Error;

use crate::{
    ast::{CallExpr, Expr, Stmt, WhileStmt},
    env::Environment,
    errors::ErrorReporter,
    loxvalue::{Callable, LoxValue, NativeFn},
    tokens::{Token, TokenType},
};

#[derive(Debug, Error)]
pub enum RuntimeError {
    // This isn't really an error :-(
    #[error("Breaking out of a loop")]
    Breaking,

    #[error("Can only call functions and classes")]
    CallOnNonCallable,

    #[error("Wrong number of function arguments")]
    CallWrongNumberOfArgs,

    #[error("Operands must be numbers")]
    OperandsMustBeNumbers,

    #[error("Operands for '+' must be numbers, or first operand must be a string")]
    PlusOperandsWrong,

    #[error("Unsupported operation")]
    UnsupportedOperation,

    #[error("Attempted to divide by zero")]
    DivideByZero,

    #[error("Undefined variable")]
    UndefinedVar(String),
}

pub struct Interpreter<'a> {
    env: Rc<RefCell<Environment>>,
    error_reporter: &'a ErrorReporter,
}

impl<'a> Interpreter<'a> {
    pub fn new(error_reporter: &'a ErrorReporter) -> Self {
        let globals = Rc::new(RefCell::new(Environment::new(None)));

        globals.borrow_mut().define(
            "clock",
            LoxValue::Callable(Callable::Native(NativeFn {
                arity: 0,
                code: Arc::new(move |_args| -> Result<LoxValue, RuntimeError> {
                    let time = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap();
                    Ok(LoxValue::Number(time.as_secs() as f64))
                }),
            })),
        );

        Interpreter {
            env: globals,
            error_reporter,
        }
    }

    pub fn interpret(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            let result = self.evaluate_stmt(&stmt);
            if result.is_err() {
                return;
            }
        }
    }

    pub fn interpret_expr(&mut self, expr: &Expr) {
        let result = self.evaluate_expr(expr);
        if let Ok(val) = result {
            println!("Result: {}", val);
        }
    }

    pub fn evaluate_stmt(&mut self, stmt: &Stmt) -> Result<(), RuntimeError> {
        match stmt {
            Stmt::Block(vec) => {
                self.execute_block(vec)?;
                Ok(())
            }
            Stmt::Break => Err(RuntimeError::Breaking),
            Stmt::Expression(e) => {
                self.evaluate_expr(e)?;
                Ok(())
            }
            Stmt::If(e) => {
                let condition = self.evaluate_expr(&e.condition)?;
                if is_truthy(&condition) {
                    self.evaluate_stmt(&e.then_branch)?;
                } else if let Some(else_branch) = &e.else_branch {
                    self.evaluate_stmt(else_branch)?;
                }
                Ok(())
            }
            Stmt::Print(e) => {
                let val = self.evaluate_expr(e)?;
                println!("{}", val);
                Ok(())
            }
            Stmt::While(WhileStmt { condition, body }) => {
                while is_truthy(&self.evaluate_expr(&condition)?) {
                    let result = self.evaluate_stmt(body);
                    if let Err(e) = result {
                        if let RuntimeError::Breaking = e {
                            return Ok(());
                        } else {
                            return Err(e);
                        }
                    }
                }
                Ok(())
            }
            Stmt::Var(vs) => {
                let value = self.evaluate_expr(vs.initializer.as_ref())?;
                self.env.borrow_mut().define(&vs.name.lexeme, value);
                Ok(())
            }
        }
    }

    fn execute_block(&mut self, stmts: &[Stmt]) -> Result<(), RuntimeError> {
        let block_env = Rc::new(RefCell::new(Environment::new(Some(self.env.clone()))));
        self.env = block_env;

        for stmt in stmts {
            let result = self.evaluate_stmt(stmt);
            if let Err(e) = result {
                self.close_scope();
                return Err(e);
            }
        }
        self.close_scope();
        Ok(())
    }

    fn close_scope(&mut self) {
        let enclosing_env = self
            .env
            .borrow()
            .enclosing()
            .expect("This environment should not be the root environment");
        self.env = enclosing_env;
    }

    fn evaluate_expr(&mut self, expr: &Expr) -> Result<LoxValue, RuntimeError> {
        match expr {
            Expr::Binary(binary) => {
                let left = self.evaluate_expr(binary.left.as_ref())?;
                let right = self.evaluate_expr(binary.right.as_ref())?;
                self.evaluate_binary(&binary.operator, &left, &right)
            }
            Expr::Call(CallExpr {
                callee,
                paren: _,
                arguments,
            }) => {
                let callee = self.evaluate_expr(&callee)?;

                let args: Vec<LoxValue> = arguments
                    .iter()
                    .map(|a| self.evaluate_expr(a).unwrap_or(LoxValue::Nil))
                    .collect();
                if let LoxValue::Callable(c) = callee {
                    if args.len() != c.arity() {
                        self.error_reporter.runtime_error(
                            0,
                            &("Expected ".to_string()
                                + &c.arity().to_string()
                                + " arguments but got "
                                + &args.len().to_string()),
                        );
                        return Err(RuntimeError::CallWrongNumberOfArgs);
                    }
                    Ok(c.call(self, &args)?)
                } else {
                    self.error_reporter
                        .runtime_error(0, &RuntimeError::CallOnNonCallable.to_string());
                    Err(RuntimeError::CallOnNonCallable)
                }
            }
            Expr::Grouping(e) => self.evaluate_expr(e.as_ref()),
            Expr::Literal(l) => Ok(LoxValue::try_from(l).unwrap_or(LoxValue::Nil)),
            Expr::Logical(e) => self.evaluate_logical(&e.left, &e.operator, &e.right),
            Expr::Unary(unary) => {
                let right = self.evaluate_expr(unary.right.as_ref())?;
                self.evaluate_unary(&unary.operator, &right)
            }
            Expr::Variable(token) => self
                .env
                .borrow()
                .get(&token.lexeme)
                .or_else(|e| self.error(&token, e)),
            Expr::Assign(assign_expr) => {
                let value = self.evaluate_expr(assign_expr.value.as_ref())?;
                self.env
                    .borrow_mut()
                    .assign(&assign_expr.name.lexeme, value.clone())
                    .or_else(|e| self.error(&assign_expr.name, e).map(|_| ()))?;
                Ok(value)
            }
        }
    }

    fn evaluate_logical(
        &mut self,
        left: &Expr,
        op: &Token,
        right: &Expr,
    ) -> Result<LoxValue, RuntimeError> {
        let left_val = self.evaluate_expr(left)?;
        if let TokenType::Or = op.token_type {
            if is_truthy(&left_val) {
                return Ok(left_val);
            }
        } else if !is_truthy(&left_val) {
            return Ok(left_val);
        }
        self.evaluate_expr(right)
    }

    fn evaluate_unary(&self, operator: &Token, right: &LoxValue) -> Result<LoxValue, RuntimeError> {
        match (&operator.token_type, &right) {
            (TokenType::Minus, &LoxValue::Number(n)) => Ok(LoxValue::Number(n * -1.0)),
            (TokenType::Bang, right) => Ok(LoxValue::Boolean(!is_truthy(&right))),
            _ => self.error(operator, RuntimeError::UnsupportedOperation),
        }
    }

    fn evaluate_binary(
        &self,
        operator: &Token,
        left: &LoxValue,
        right: &LoxValue,
    ) -> Result<LoxValue, RuntimeError> {
        match (&operator.token_type, &left, &right) {
            (TokenType::Minus, &LoxValue::Number(nl), &LoxValue::Number(nr)) => {
                Ok(LoxValue::Number(nl - nr))
            }
            (TokenType::Slash, &LoxValue::Number(nl), &LoxValue::Number(nr)) => {
                if *nr == 0.0 {
                    self.error(operator, RuntimeError::DivideByZero)
                } else {
                    Ok(LoxValue::Number(nl / nr))
                }
            }
            (TokenType::Star, &LoxValue::Number(nl), &LoxValue::Number(nr)) => {
                Ok(LoxValue::Number(nl * nr))
            }
            (TokenType::Plus, &LoxValue::Number(nl), &LoxValue::Number(nr)) => {
                Ok(LoxValue::Number(nl + nr))
            }
            (TokenType::Plus, &LoxValue::String(sl), &LoxValue::String(sr)) => {
                let mut s = String::new();
                s.push_str(&sl);
                s.push_str(&sr);
                Ok(LoxValue::String(s))
            }
            (TokenType::Plus, &LoxValue::String(sl), &non_string) => {
                let mut s = String::new();
                s.push_str(&sl);
                s.push_str(&non_string.to_string());
                Ok(LoxValue::String(s))
            }
            (TokenType::Greater, &LoxValue::Number(nl), &LoxValue::Number(nr)) => {
                Ok(LoxValue::Boolean(nl > nr))
            }
            (TokenType::GreaterEqual, &LoxValue::Number(nl), &LoxValue::Number(nr)) => {
                Ok(LoxValue::Boolean(nl >= nr))
            }
            (TokenType::Less, &LoxValue::Number(nl), &LoxValue::Number(nr)) => {
                Ok(LoxValue::Boolean(nl < nr))
            }
            (TokenType::LessEqual, &LoxValue::Number(nl), &LoxValue::Number(nr)) => {
                Ok(LoxValue::Boolean(nl <= nr))
            }
            (TokenType::BangEqual, left, right) => Ok(LoxValue::Boolean(left != right)),
            (TokenType::EqualEqual, left, right) => Ok(LoxValue::Boolean(left == right)),

            // Handle invalid cases
            (TokenType::Minus, _, _) => self.error(operator, RuntimeError::OperandsMustBeNumbers),
            (TokenType::Slash, _, _) => self.error(operator, RuntimeError::OperandsMustBeNumbers),
            (TokenType::Star, _, _) => self.error(operator, RuntimeError::OperandsMustBeNumbers),
            (TokenType::Plus, _, _) => self.error(operator, RuntimeError::PlusOperandsWrong),
            (TokenType::Greater, _, _) => self.error(operator, RuntimeError::OperandsMustBeNumbers),
            (TokenType::GreaterEqual, _, _) => {
                self.error(operator, RuntimeError::OperandsMustBeNumbers)
            }
            (TokenType::Less, _, _) => self.error(operator, RuntimeError::OperandsMustBeNumbers),
            (TokenType::LessEqual, _, _) => {
                self.error(operator, RuntimeError::OperandsMustBeNumbers)
            }
            _ => self.error(operator, RuntimeError::UnsupportedOperation),
        }
    }

    fn error(&self, token: &Token, error: RuntimeError) -> Result<LoxValue, RuntimeError> {
        self.error_reporter
            .runtime_error(token.line, &error.to_string());
        Err(error)
    }
}

fn is_truthy(val: &LoxValue) -> bool {
    match val {
        LoxValue::Nil => false,
        LoxValue::Boolean(false) => false,
        _ => true,
    }
}
