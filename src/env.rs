use std::collections::HashMap;

use crate::interpreter::{LoxValue, RuntimeError};

pub struct Environment {
    values: HashMap<String, LoxValue>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            values: HashMap::new(),
        }
    }

    pub fn define(&mut self, name: &str, value: LoxValue) {
        self.values.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Result<LoxValue, RuntimeError> {
        if let Some(val) = self.values.get(&name.to_string()) {
            Ok(val.clone())
        } else {
            Err(RuntimeError::UndefinedVar(name.to_string()))
        }
    }

    pub fn assign(&mut self, name: &str, value: LoxValue) -> Result<(), RuntimeError> {
        let nm = name.to_string();
        if self.values.contains_key(&nm) {
            self.values.insert(nm, value);
            Ok(())
        } else {
            Err(RuntimeError::UndefinedVar(nm))
        }
    }
}
