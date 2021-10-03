use crate::value::Value;
use crate::gc::Obj;

struct Entry {
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
