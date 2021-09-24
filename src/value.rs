use std::fmt::{Display, Error, Formatter};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Nil,
    Number(f32),
    Boolean(bool),
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Number(n) => n.fmt(f),
            Value::Boolean(b) => b.fmt(f),
        }
    }
}

pub type ValueArray = Vec<Value>;

pub fn print_value(value: Value) -> String {
    return format!("{}", value);
}
