use std::{cell::RefCell, convert::TryFrom, fmt::Display, rc::Rc, sync::Arc};

use crate::{
    ast::FunctionStmt,
    env::Environment,
    interpreter::{Interpreter, RuntimeError},
    tokens::TokenLiteral,
};

#[derive(Clone, Debug, PartialEq)]
pub enum LoxValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Callable(Callable),
}

impl Display for LoxValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoxValue::Nil => {
                f.write_str("Nil")?;
            }
            LoxValue::Boolean(b) => {
                if *b {
                    f.write_str("true")?;
                } else {
                    f.write_str("false")?;
                }
            }
            LoxValue::Callable(_) => {
                f.write_str("(callable)")?;
            }
            LoxValue::Number(n) => {
                f.write_fmt(format_args!("{}", n))?;
            }
            LoxValue::String(s) => {
                f.write_str(&s)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Callable {
    Function(FunctionStmt),
    Native(NativeFn),
}

impl Callable {
    pub fn call(
        &self,
        interpreter: &mut Interpreter,
        args: &[LoxValue],
    ) -> Result<LoxValue, RuntimeError> {
        match &self {
            Callable::Native(nfn) => nfn.call(args),
            Callable::Function(FunctionStmt {
                name: _,
                params,
                body,
            }) => {
                let env = Rc::new(RefCell::new(Environment::new(Some(interpreter.globals()))));
                if args.len() != params.len() {
                    return Err(RuntimeError::CallWrongNumberOfArgs);
                }
                for i in 0..args.len() {
                    env.borrow_mut().define(&params[i].lexeme, args[i].clone());
                }
                match interpreter.execute_block(body, env) {
                    Ok(()) => Ok(LoxValue::Nil),
                    Err(RuntimeError::Return(val)) => Ok(val),
                    Err(e) => Err(e),
                }
            }
        }
    }

    pub fn arity(&self) -> usize {
        match &self {
            Callable::Native(nfn) => nfn.arity,
            Callable::Function(FunctionStmt {
                name: _,
                params,
                body: _,
            }) => params.len(),
        }
    }
}

impl Display for Callable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Callable::Function(FunctionStmt {
                name,
                params: _,
                body: _,
            }) => {
                f.write_str("fun ")?;
                f.write_str(&name.lexeme)
            }
            Callable::Native(_) => f.write_str("<builtin function>"),
        }
    }
}

#[derive(Clone)]
pub struct NativeFn {
    pub arity: usize,
    pub code: Arc<dyn Fn(&[LoxValue]) -> Result<LoxValue, RuntimeError>>,
}

impl NativeFn {
    pub fn call(&self, args: &[LoxValue]) -> Result<LoxValue, RuntimeError> {
        if args.len() != self.arity {
            return Err(RuntimeError::CallWrongNumberOfArgs);
        }
        (self.code)(args)
    }
}

impl std::fmt::Debug for NativeFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeFn")
            .field("arity", &self.arity)
            .finish()
    }
}

impl PartialEq for Callable {
    // Two native functions are never equal. This might not be right long-term...
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

pub struct LoxValueError {}

impl TryFrom<&TokenLiteral> for LoxValue {
    type Error = LoxValueError;

    fn try_from(l: &TokenLiteral) -> Result<Self, Self::Error> {
        match l {
            // NB: Not sure translating no token literal value to nil is kosher,
            // but I *think* this is following the book for now at least.
            TokenLiteral::None => Ok(LoxValue::Nil),
            TokenLiteral::True => Ok(LoxValue::Boolean(true)),
            TokenLiteral::False => Ok(LoxValue::Boolean(false)),
            TokenLiteral::Nil => Ok(LoxValue::Nil),
            TokenLiteral::String(s) => Ok(LoxValue::String(s.clone())),
            TokenLiteral::Number(n) => Ok(LoxValue::Number(*n)),
        }
    }
}
