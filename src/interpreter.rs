use std::{
    cell::RefCell, collections::HashMap, convert::TryFrom, rc::Rc, sync::Arc, time::SystemTime,
};
use thiserror::Error;

use crate::{
    ast::{CallExpr, ClassStmt, Expr, GetExpr, ReturnStmt, Stmt, WhileStmt},
    env::Environment,
    errors::ErrorReporter,
    loxvalue::{Function, LoxCallable, LoxClass, LoxRef, LoxValue, NativeFn},
    tokens::{Token, TokenType},
};

#[derive(Debug, Error)]
pub enum RuntimeError<'a> {
    // This isn't really an error :-(
    #[error("Breaking out of a loop")]
    Breaking,

    // Nor this :-(
    #[error("Returning from function")]
    Return(LoxValue<'a>),

    #[error("Can only call functions and classes")]
    CallOnNonCallable,

    #[error("Wrong number of function arguments")]
    CallWrongNumberOfArgs,

    #[error("Only instances have fields")]
    FieldAccessOnNonInstance,

    #[error("Operands must be numbers")]
    OperandsMustBeNumbers,

    #[error("Operands for '+' must be numbers, or first operand must be a string")]
    PlusOperandsWrong,

    #[error("Undefined property")]
    UndefinedProperty(String),

    #[error("Unsupported operation")]
    UnsupportedOperation,

    #[error("Attempted to divide by zero")]
    DivideByZero,

    #[error("Undefined variable")]
    UndefinedVar(String),
}

pub struct Interpreter<'a, 'b> {
    env: Rc<RefCell<Environment<'b>>>,
    globals: Rc<RefCell<Environment<'b>>>,
    locals: HashMap<*const Expr, usize>,
    error_reporter: &'a ErrorReporter,
}

impl<'a, 'b> Interpreter<'a, 'b> {
    pub fn new(error_reporter: &'a ErrorReporter) -> Self {
        let globals = Rc::new(RefCell::new(Environment::new(None)));

        globals.borrow_mut().define(
            "clock",
            LoxValue::Ref(Rc::new(RefCell::new(LoxRef::Function(Function::Native(
                NativeFn {
                    arity: 0,
                    code: Arc::new(move |_args| -> Result<LoxValue, RuntimeError> {
                        let time = SystemTime::now()
                            .duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap();
                        Ok(LoxValue::Number(time.as_secs() as f64))
                    }),
                },
            ))))),
        );

        Interpreter {
            env: globals.clone(),
            globals,
            locals: HashMap::new(),
            error_reporter,
        }
    }

    pub fn interpret(&mut self, stmts: &'b [Stmt]) {
        // println!("Locals from resolver: {:?}", self.locals);
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

    pub fn evaluate_stmt(&mut self, stmt: &'b Stmt) -> Result<(), RuntimeError<'b>> {
        match stmt {
            Stmt::Block(vec) => {
                let block_env = Rc::new(RefCell::new(Environment::new(Some(self.env.clone()))));
                self.execute_block(vec, block_env)?;
                Ok(())
            }
            Stmt::Break => Err(RuntimeError::Breaking),
            Stmt::Class(ClassStmt { name, methods: _ }) => {
                let mut env = self.env.borrow_mut();
                env.define(&name.lexeme, LoxValue::Nil);
                let c = LoxClass::new(name.lexeme.clone());
                env.assign(
                    &name.lexeme,
                    LoxValue::Ref(Rc::new(RefCell::new(LoxRef::Class(c)))),
                )
            }
            Stmt::Expression(e) => {
                self.evaluate_expr(e)?;
                Ok(())
            }
            Stmt::Function(stmt) => {
                let callable = Function::new_function(&stmt, self.env.clone());
                self.env.borrow_mut().define(
                    &stmt.name.lexeme,
                    LoxValue::Ref(Rc::new(RefCell::new(LoxRef::Function(callable)))),
                );
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
            Stmt::Return(ReturnStmt { keyword: _, value }) => {
                let val = self.evaluate_expr(value)?;
                Err(RuntimeError::Return(val))
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

    pub fn execute_block(
        &mut self,
        stmts: &'b [Stmt],
        env: Rc<RefCell<Environment<'b>>>,
    ) -> Result<(), RuntimeError<'b>> {
        let previous_env = self.env.clone();
        self.env = env;
        for stmt in stmts {
            let result = self.evaluate_stmt(stmt);
            if let Err(e) = result {
                self.env = previous_env;
                return Err(e);
            }
        }
        self.env = previous_env;
        Ok(())
    }

    fn evaluate_expr(&mut self, expr: &Expr) -> Result<LoxValue<'b>, RuntimeError<'b>> {
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
                if let LoxValue::Ref(r) = callee {
                    match &*r.borrow() {
                        LoxRef::Function(f) => {
                            let none: Option<Rc<RefCell<LoxRef>>> = None;
                            self.evaluate_call(none, &args, f)
                        }
                        LoxRef::Class(c) => self.evaluate_call(Some(r.clone()), &args, c),
                        LoxRef::Instance(_) => {
                            self.error_reporter
                                .runtime_error(0, &RuntimeError::CallOnNonCallable.to_string());
                            Err(RuntimeError::CallOnNonCallable)
                        }
                    }
                } else {
                    self.error_reporter
                        .runtime_error(0, &RuntimeError::CallOnNonCallable.to_string());
                    Err(RuntimeError::CallOnNonCallable)
                }
            }
            Expr::Get(GetExpr { name, object }) => {
                let object = self.evaluate_expr(object)?;
                if let LoxValue::Ref(r) = &object {
                    if let LoxRef::Instance(i) = &*r.borrow() {
                        return i.get(&name.lexeme).map_err(|_| {
                            self.error(&name, RuntimeError::UndefinedProperty(name.lexeme.clone()))
                                .unwrap_err()
                        });
                    }
                }
                Err(RuntimeError::FieldAccessOnNonInstance)
            }
            Expr::Grouping(e) => self.evaluate_expr(e.as_ref()),
            Expr::Literal(l) => Ok(LoxValue::try_from(l).unwrap_or(LoxValue::Nil)),
            Expr::Logical(e) => self.evaluate_logical(&e.left, &e.operator, &e.right),
            Expr::Set(e) => {
                let val = self.evaluate_expr(&*e.object)?;
                if let LoxValue::Ref(r) = val {
                    if let LoxRef::Instance(ref mut i) = &mut *r.borrow_mut() {
                        let val = self.evaluate_expr(&*e.value)?;
                        i.set(&e.name.lexeme, val.clone());
                        return Ok(val);
                    }
                }

                Err(RuntimeError::FieldAccessOnNonInstance)
            }
            Expr::Unary(unary) => {
                let right = self.evaluate_expr(unary.right.as_ref())?;
                self.evaluate_unary(&unary.operator, &right)
            }
            Expr::Variable(token) => self.lookup_variable(token, expr),
            Expr::Assign(assign_expr) => {
                let value = self.evaluate_expr(assign_expr.value.as_ref())?;
                // println!("Lookup for name {} with ptr {:?}", assign_expr.name.lexeme, assign_expr as *const Expr);
                if let Some(distance) = self.locals.get(&(expr as *const Expr)) {
                    // println!("Assigning at distance {}", distance);
                    self.env
                        .borrow_mut()
                        .assign_at(*distance, &assign_expr.name.lexeme, value.clone())
                        .or_else(|e| self.error(&assign_expr.name, e).map(|_| ()))?;
                } else {
                    // println!("Assigning global: {}", &assign_expr.name.lexeme);
                    self.globals
                        .borrow_mut()
                        .assign(&assign_expr.name.lexeme, value.clone())
                        .or_else(|e| self.error(&assign_expr.name, e).map(|_| ()))?;
                }

                Ok(value)
            }
        }
    }

    fn evaluate_call(
        &mut self,
        this: Option<Rc<RefCell<LoxRef<'b>>>>,
        args: &[LoxValue<'b>],
        callable: &impl LoxCallable<'b>,
    ) -> Result<LoxValue<'b>, RuntimeError<'b>> {
        if args.len() != callable.arity() {
            self.error_reporter.runtime_error(
                0,
                &("Expected ".to_string()
                    + &callable.arity().to_string()
                    + " arguments but got "
                    + &args.len().to_string()),
            );
            return Err(RuntimeError::CallWrongNumberOfArgs);
        }
        callable.call(this, self, &args)
    }

    fn evaluate_logical(
        &mut self,
        left: &Expr,
        op: &Token,
        right: &Expr,
    ) -> Result<LoxValue<'b>, RuntimeError<'b>> {
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

    fn evaluate_unary(
        &self,
        operator: &Token,
        right: &LoxValue,
    ) -> Result<LoxValue<'b>, RuntimeError<'b>> {
        match (&operator.token_type, &right) {
            (TokenType::Minus, &LoxValue::Number(n)) => Ok(LoxValue::Number(n * -1.0)),
            (TokenType::Bang, right) => Ok(LoxValue::Boolean(!is_truthy(&right))),
            _ => self.error(operator, RuntimeError::UnsupportedOperation),
        }
    }

    fn evaluate_binary(
        &self,
        operator: &Token,
        left: &LoxValue<'b>,
        right: &LoxValue<'b>,
    ) -> Result<LoxValue<'b>, RuntimeError<'b>> {
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

    fn error(
        &self,
        token: &Token,
        error: RuntimeError<'b>,
    ) -> Result<LoxValue<'b>, RuntimeError<'b>> {
        self.error_reporter
            .runtime_error(token.line, &error.to_string());
        Err(error)
    }

    pub fn resolve(&mut self, expr: &Expr, distance: usize) {
        // println!("Resolving expr with ptr {:?} and distance {}", expr as *const Expr, distance);
        self.locals.insert(expr as *const Expr, distance);
    }

    fn lookup_variable(
        &mut self,
        name: &Token,
        expr: &Expr,
    ) -> Result<LoxValue<'b>, RuntimeError<'b>> {
        // println!("Lookup for name {} with ptr {:?}", name.lexeme, expr as *const Expr);
        if let Some(distance) = self.locals.get(&(expr as *const Expr)) {
            self.env.borrow_mut().get_at(*distance, &name.lexeme)
        } else {
            // println!("Have too look up global for {}", name.lexeme);
            self.globals.borrow_mut().get(&name.lexeme)
        }
    }
}

fn is_truthy(val: &LoxValue) -> bool {
    match val {
        LoxValue::Nil => false,
        LoxValue::Boolean(false) => false,
        _ => true,
    }
}
