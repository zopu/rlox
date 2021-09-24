use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{interpreter::RuntimeError, loxvalue::LoxValue};

#[derive(Debug)]
pub struct Environment<'a> {
    enclosing: Option<Rc<RefCell<Environment<'a>>>>,
    values: HashMap<String, LoxValue<'a>>,
}

impl<'a> Environment<'a> {
    pub fn new(enclosing: Option<Rc<RefCell<Environment<'a>>>>) -> Self {
        Environment {
            enclosing,
            values: HashMap::new(),
        }
    }

    pub fn enclosing(&self) -> Option<Rc<RefCell<Environment<'a>>>> {
        self.enclosing.clone()
    }

    pub fn define(&mut self, name: &str, value: LoxValue<'a>) {
        self.values.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Result<LoxValue<'a>, RuntimeError<'a>> {
        if let Some(val) = self.values.get(&name.to_string()) {
            Ok(val.clone())
        } else if let Some(parent) = &self.enclosing {
            (*parent).borrow().get(name)
        } else {
            Err(RuntimeError::UndefinedVar(name.to_string()))
        }
    }

    pub fn get_at(&self, distance: usize, name: &str) -> Result<LoxValue<'a>, RuntimeError<'a>> {
        if distance == 0 {
            self.get(name)
        } else if let Some(env) = &self.enclosing {
            env.borrow_mut().get_at(distance - 1, name)
        } else {
            panic!("Resolver calculated distance greater than stack size");
        }
    }

    pub fn assign(&mut self, name: &str, value: LoxValue<'a>) -> Result<(), RuntimeError<'a>> {
        let nm = name.to_string();
        if self.values.contains_key(&nm) {
            self.values.insert(nm, value);
            Ok(())
        } else if let Some(parent) = &self.enclosing {
            (**parent).borrow_mut().assign(name, value)
        } else {
            Err(RuntimeError::UndefinedVar(nm))
        }
    }

    pub fn assign_at(
        &mut self,
        distance: usize,
        name: &str,
        value: LoxValue<'a>,
    ) -> Result<(), RuntimeError<'a>> {
        if distance == 0 {
            self.assign(name, value)
        } else if let Some(env) = &self.enclosing {
            env.borrow_mut().assign_at(distance - 1, name, value)
        } else {
            panic!("Resolver calculated distance greater than stack size");
        }
    }
}
