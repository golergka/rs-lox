use crate::value::Value;

struct Entry {
    key: *const String,
    value: Value,
}

struct Table {
    data: Vec<Entry>,
}

impl Table {
    fn new() -> Self {
        Self {
            data: Vec::new(),
        }
    }

}
