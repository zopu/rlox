use core::panic;
use std::{cell::RefCell, collections::HashMap, convert::TryFrom, fmt::Display, rc::Rc, sync::Arc};
use thiserror::Error;

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
    Ref(Rc<RefCell<LoxRef<'a>>>),
}

impl<'a> Display for LoxValue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoxValue::Nil => f.write_str("Nil"),
            LoxValue::Boolean(b) => {
                if *b {
                    f.write_str("true")
                } else {
                    f.write_str("false")
                }
            }
            LoxValue::Ref(r) => r.borrow().fmt(f),
            LoxValue::Number(n) => f.write_fmt(format_args!("{}", n)),
            LoxValue::String(s) => f.write_str(&s),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum LoxRef<'a> {
    Function(Function<'a>),
    Class(LoxClass),
    Instance(LoxInstance<'a>),
}

impl<'a> Display for LoxRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoxRef::Function(_) => f.write_str("(function)"),
            LoxRef::Class(c) => f.write_str(&c.name),
            LoxRef::Instance(inst) => {
                f.write_str(&inst.class_name())?;
                f.write_str(" instance")
            }
        }
    }
}

pub trait LoxCallable<'a> {
    fn call(
        &self,
        this: Option<Rc<RefCell<LoxRef<'a>>>>,
        interpreter: &mut Interpreter<'_, 'a>,
        args: &[LoxValue<'a>],
    ) -> Result<LoxValue<'a>, RuntimeError<'a>>;

    fn arity(&self) -> usize;
}

#[derive(Clone, Debug)]
pub enum Function<'a> {
    UserDefined(UserFunction<'a>),
    Native(NativeFn<'a>),
}

impl<'a> Function<'a> {
    pub fn new_function(
        declaration: &'a FunctionStmt,
        closure: Rc<RefCell<Environment<'a>>>,
    ) -> Function<'a> {
        Function::UserDefined(UserFunction {
            code: declaration,
            closure,
        })
    }
}

impl<'a> LoxCallable<'a> for Function<'a> {
    fn call(
        &self,
        _this: Option<Rc<RefCell<LoxRef<'a>>>>,
        interpreter: &mut Interpreter<'_, 'a>,
        args: &[LoxValue<'a>],
    ) -> Result<LoxValue<'a>, RuntimeError<'a>> {
        match &self {
            Function::Native(nfn) => nfn.call(args),
            Function::UserDefined(UserFunction {
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

    fn arity(&self) -> usize {
        match &self {
            Function::Native(nfn) => nfn.arity,
            Function::UserDefined(UserFunction {
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

impl<'a> Display for Function<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::UserDefined(UserFunction {
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
            Function::Native(_) => f.write_str("<builtin function>"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserFunction<'a> {
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

impl<'a> PartialEq for Function<'a> {
    // Two native functions are never equal. This might not be right long-term...
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoxClass {
    name: String,
}

impl LoxClass {
    pub fn new(name: String) -> LoxClass {
        LoxClass { name }
    }
}

impl<'a> LoxCallable<'a> for LoxClass {
    fn call(
        &self,
        this: Option<Rc<RefCell<LoxRef<'a>>>>,
        _interpreter: &mut Interpreter<'_, 'a>,
        _args: &[LoxValue<'a>],
    ) -> Result<LoxValue<'a>, RuntimeError<'a>> {
        if let Some(this) = this {
            if let LoxRef::Class(_) = *this.borrow() {
                return Ok(LoxValue::Ref(Rc::new(RefCell::new(LoxRef::Instance(
                    LoxInstance::new(this.clone()),
                )))));
            }
        }
        panic!("Should have 'this' when calling a class object");
    }

    fn arity(&self) -> usize {
        0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoxInstance<'a> {
    // Ugly that we don't strongly type this to LoxClass vs LoxRef here.
    // That's because we're taking the Rc<RefCell<>> from the LoxValue.
    class: Rc<RefCell<LoxRef<'a>>>,
    fields: HashMap<String, LoxValue<'a>>,
}

#[derive(Debug, Error)]
pub enum LoxInstanceError {
    #[error("Undefined property")]
    LookupError(String),
}

impl<'a> LoxInstance<'a> {
    pub fn new(class: Rc<RefCell<LoxRef<'a>>>) -> LoxInstance<'a> {
        LoxInstance {
            class,
            fields: HashMap::new(),
        }
    }
    pub fn class_name(&self) -> String {
        if let LoxRef::Class(c) = &*self.class.borrow() {
            c.name.clone()
        } else {
            panic!("Instance's class is not a class!");
        }
    }

    pub fn get(&self, name: &str) -> Result<LoxValue<'a>, LoxInstanceError> {
        match self.fields.get(name) {
            Some(val) => Ok(val.clone()),
            None => Err(LoxInstanceError::LookupError(name.to_string())),
        }
    }

    pub fn set(&mut self, name: &str, value: LoxValue<'a>) {
        self.fields.insert(name.to_string(), value);
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
