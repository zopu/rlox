use std::{convert::TryFrom, fmt::Display};
use thiserror::Error;

use crate::{
    errors::ErrorReporter,
    expr::Expr,
    tokens::{Token, TokenLiteral, TokenType},
};

#[derive(Debug, PartialEq)]
pub enum LoxValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(String),
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

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("Operands must be numbers")]
    OperandsMustBeNumbers,

    #[error("Operands for '+' must be numbers, or first operand must be a string")]
    PlusOperandsWrong,

    #[error("Unsupported operation")]
    UnsupportedOperation,
}

pub struct Interpreter<'a> {
    error_reporter: &'a ErrorReporter,
}

impl<'a> Interpreter<'a> {
    pub fn new(error_reporter: &'a ErrorReporter) -> Self {
        Interpreter { error_reporter }
    }

    pub fn interpret(&self, expr: &Expr) -> Result<LoxValue, RuntimeError> {
        match expr {
            Expr::Binary(binary) => {
                let left = self.interpret(binary.left.as_ref())?;
                let right = self.interpret(binary.right.as_ref())?;
                self.evaluate_binary(&binary.operator, &left, &right)
            }
            Expr::Grouping(e) => self.interpret(e.as_ref()),
            Expr::Literal(l) => Ok(LoxValue::try_from(l).unwrap_or(LoxValue::Nil)),
            Expr::Unary(unary) => {
                let right = self.interpret(unary.right.as_ref())?;
                self.evaluate_unary(&unary.operator, &right)
            }
        }
    }

    fn evaluate_unary(&self, operator: &Token, right: &LoxValue) -> Result<LoxValue, RuntimeError> {
        match (&operator.token_type, &right) {
            (TokenType::Minus, &LoxValue::Number(n)) => Ok(LoxValue::Number(n * -1.0)),
            (TokenType::Bang, right) => Ok(LoxValue::Boolean(!is_truthy(&right))),
            _ => Err(RuntimeError::UnsupportedOperation),
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
                Ok(LoxValue::Number(nl / nr))
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
        LoxValue::Boolean(true) => true,
        _ => false,
    }
}
