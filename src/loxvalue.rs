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
    Class(LoxClass<'a>),
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

    pub fn bind(&self, this_ref: Rc<RefCell<LoxRef<'a>>>) -> Function<'a> {
        match self {
            Function::UserDefined(f) => Function::UserDefined(f.bind(this_ref)),
            Function::Native(_) => self.clone(),
        }
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
            Function::UserDefined(f) => f.code.params.len(),
        }
    }
}

impl<'a> Display for Function<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::UserDefined(fun) => {
                f.write_str("fun ")?;
                f.write_str(&fun.code.name.lexeme)
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

impl<'a> UserFunction<'a> {
    pub fn bind(&self, this_ref: Rc<RefCell<LoxRef<'a>>>) -> UserFunction<'a> {
        let mut new_fun = self.clone();
        new_fun.closure = Rc::new(RefCell::new(Environment::new(Some(self.closure.clone()))));
        new_fun
            .closure
            .borrow_mut()
            .define("this", LoxValue::Ref(this_ref));
        new_fun
    }
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
pub struct LoxClass<'a> {
    name: String,
    methods: HashMap<String, LoxValue<'a>>,
}

impl<'a> LoxClass<'a> {
    // NB probably should be safer and assert that all these LoxValues are actually functions here.
    pub fn new(name: String, methods: HashMap<String, LoxValue<'a>>) -> LoxClass {
        LoxClass { name, methods }
    }

    pub fn find_method(&self, name: &str) -> Option<LoxValue<'a>> {
        self.methods.get(name).cloned()
    }
}

impl<'a> LoxCallable<'a> for LoxClass<'a> {
    fn call(
        &self,
        this: Option<Rc<RefCell<LoxRef<'a>>>>,
        interpreter: &mut Interpreter<'_, 'a>,
        args: &[LoxValue<'a>],
    ) -> Result<LoxValue<'a>, RuntimeError<'a>> {
        if let Some(this) = this {
            if let LoxRef::Class(_) = *this.borrow() {
                let instance_ref = Rc::new(RefCell::new(LoxRef::Instance(LoxInstance::new(
                    this.clone(),
                ))));
                if let Some(loxval) = self.find_method("init") {
                    if let LoxValue::Ref(r) = loxval {
                        if let LoxRef::Function(f) = &*r.borrow() {
                            let bound_f = f.bind(instance_ref.clone());
                            bound_f.call(Some(this.clone()), interpreter, args)?;
                            return Ok(LoxValue::Ref(instance_ref));
                        }
                    }
                    panic!("Method is not a function");
                } else {
                    return Ok(LoxValue::Ref(instance_ref));
                }
            }
        }
        panic!("Should have 'this' when calling a class object");
    }

    fn arity(&self) -> usize {
        if let Some(loxval) = self.find_method("init") {
            if let LoxValue::Ref(r) = loxval {
                if let LoxRef::Function(f) = &*r.borrow() {
                    return f.arity();
                }
            }
            panic!("Method is not a function");
        }
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

    pub fn get<'b>(
        &self,
        self_ref: Rc<RefCell<LoxRef<'a>>>,
        name: &'b str,
    ) -> Result<LoxValue<'a>, LoxInstanceError> {
        if let Some(val) = self.fields.get(name) {
            return Ok(val.clone());
        }

        if let LoxRef::Class(c) = &*self.class.borrow() {
            if let Some(LoxValue::Ref(r)) = c.find_method(name) {
                if let LoxRef::Function(f) = &*r.borrow() {
                    return Ok(LoxValue::Ref(Rc::new(RefCell::new(LoxRef::Function(
                        f.bind(self_ref),
                    )))));
                }
            }
        }

        Err(LoxInstanceError::LookupError(name.to_string()))
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
