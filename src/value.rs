pub type Value = f32;

pub type ValueArray = Vec<Value>;

pub fn print_value(value: Value) -> String {
    return format!("{}", value);
}