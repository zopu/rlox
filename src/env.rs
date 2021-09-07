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
}
