use std::{cell::RefCell, convert::TryFrom, fmt::Display, rc::Rc, sync::Arc};

use crate::{
    ast::FunctionStmt,
    env::Environment,
    interpreter::{Interpreter, RuntimeError},
    tokens::TokenLiteral,
};

#[derive(Clone, Debug, PartialEq)]
pub enum LoxValue<'a> {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
    Callable(Callable<'a>),
}

impl<'a> Display for LoxValue<'a> {
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
pub enum Callable<'a> {
    Function(Function<'a>),
    Native(NativeFn<'a>),
}

impl<'a> Callable<'a> {
    pub fn new_function(
        declaration: &'a FunctionStmt,
        closure: Rc<RefCell<Environment<'a>>>,
    ) -> Callable<'a> {
        Callable::Function(Function {
            code: declaration,
            closure,
        })
    }

    pub fn call(
        &self,
        interpreter: &mut Interpreter<'_, 'a>,
        args: &[LoxValue<'a>],
    ) -> Result<LoxValue<'a>, RuntimeError<'a>> {
        match &self {
            Callable::Native(nfn) => nfn.call(args),
            Callable::Function(Function {
                code:
                    FunctionStmt {
                        name: _,
                        params,
                        body,
                    },
                closure,
            }) => {
                let env = Rc::new(RefCell::new(Environment::new(Some(closure.clone()))));
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
            Callable::Function(Function {
                code:
                    FunctionStmt {
                        name: _,
                        params,
                        body: _,
                    },
                closure: _,
            }) => params.len(),
        }
    }
}

impl<'a> Display for Callable<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Callable::Function(Function {
                code:
                    FunctionStmt {
                        name,
                        params: _,
                        body: _,
                    },
                closure: _,
            }) => {
                f.write_str("fun ")?;
                f.write_str(&name.lexeme)
            }
            Callable::Native(_) => f.write_str("<builtin function>"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Function<'a> {
    pub code: &'a FunctionStmt,
    closure: Rc<RefCell<Environment<'a>>>,
}

#[derive(Clone)]
pub struct NativeFn<'a> {
    pub arity: usize,
    pub code: Arc<dyn Fn(&[LoxValue]) -> Result<LoxValue<'a>, RuntimeError<'a>>>,
}

impl<'a> NativeFn<'a> {
    pub fn call(&self, args: &[LoxValue]) -> Result<LoxValue<'a>, RuntimeError<'a>> {
        if args.len() != self.arity {
            return Err(RuntimeError::CallWrongNumberOfArgs);
        }
        (self.code)(args)
    }
}

impl<'a> std::fmt::Debug for NativeFn<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeFn")
            .field("arity", &self.arity)
            .finish()
    }
}

impl<'a> PartialEq for Callable<'a> {
    // Two native functions are never equal. This might not be right long-term...
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

pub struct LoxValueError {}

impl<'a> TryFrom<&TokenLiteral> for LoxValue<'a> {
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
