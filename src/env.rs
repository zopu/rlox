use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::interpreter::{LoxValue, RuntimeError};

pub struct Environment {
    enclosing: Option<Rc<RefCell<Environment>>>,
    values: HashMap<String, LoxValue>,
}

impl Environment {
    pub fn new(enclosing: Option<Rc<RefCell<Environment>>>) -> Self {
        Environment {
            enclosing,
            values: HashMap::new(),
        }
    }

    pub fn enclosing(&self) -> Option<Rc<RefCell<Environment>>> {
        self.enclosing.clone()
    }

    pub fn define(&mut self, name: &str, value: LoxValue) {
        self.values.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Result<LoxValue, RuntimeError> {
        if let Some(val) = self.values.get(&name.to_string()) {
            Ok(val.clone())
        } else if let Some(parent) = &self.enclosing {
            (*parent).borrow().get(name)
        } else {
            Err(RuntimeError::UndefinedVar(name.to_string()))
        }
    }

    pub fn assign(&mut self, name: &str, value: LoxValue) -> Result<(), RuntimeError> {
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
}
