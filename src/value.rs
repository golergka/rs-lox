use crate::gc::ObjRef;
use std::fmt::{Display, Error, Formatter};
use Value::*;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Nil,
    Number(f32),
    Boolean(bool),
    Object(ObjRef)
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(n) => n.fmt(f),
            Value::Boolean(b) => b.fmt(f),
            Value::Object(o) => o.fmt(f)
        }
    }
}

pub type ValueArray = Vec<Value>;

pub fn is_falsey(value: Value) -> bool {
    return match value {
        Nil | Value::Boolean(false) => true,
        _ => false,
    };
}

pub fn are_equal(a: Value, b: Value) -> bool {
    return match (a, b) {
        (Nil, Nil) => true,
        (Number(a), Number(b)) => a == b,
        (Boolean(a), Boolean(b)) => a == b,
        _ => false,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_falsey() {
        assert_eq!(is_falsey(Nil), true);
        assert_eq!(is_falsey(Value::Number(0.0)), false);
        assert_eq!(is_falsey(Value::Boolean(false)), true);
        assert_eq!(is_falsey(Value::Boolean(true)), false);
    }

    #[test]
    fn test_are_equal() {
        assert_eq!(are_equal(Nil, Nil), true);
        assert_eq!(are_equal(Value::Number(0.0), Value::Number(0.0)), true);
        assert_eq!(are_equal(Value::Boolean(false), Value::Boolean(false)), true);
        assert_eq!(are_equal(Value::Boolean(true), Value::Boolean(true)), true);
        assert_eq!(are_equal(Value::Number(0.0), Value::Boolean(false)), false);
        assert_eq!(are_equal(Value::Number(0.0), Value::Boolean(true)), false);
        // Comparisons between different types are always false
        assert_eq!(are_equal(Value::Boolean(false), Value::Number(0.0)), false);
        assert_eq!(are_equal(Value::Boolean(true), Value::Number(0.0)), false);
        assert_eq!(are_equal(Value::Boolean(true), Nil), false);
        assert_eq!(are_equal(Value::Boolean(false), Nil), false);
        assert_eq!(are_equal(Value::Number(0.0), Nil), false);
    }
}