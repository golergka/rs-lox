use crate::gc::ObjString;
use crate::value::Value;
use std::alloc::{self, dealloc, Layout};
use std::ptr::{self, null_mut};

#[derive(Debug)]
struct Entry {
    key: *const ObjString,
    value: Value,
}

pub struct Table {
    ptr: *mut Entry,
    cap: usize,
    len: usize,
}

const TABLE_MAX_LOAD: f64 = 0.75;

unsafe fn find_entry(ptr: *mut Entry, cap: usize, key: *const ObjString) -> *mut Entry {
    let mut index = (*key).get_hash() % cap;
    loop {
        let entry = ptr.offset(index as isize);
        if (*entry).key == key || (*entry).key.is_null() {
            return entry;
        }
        index = (index + 1) % cap;
    }
}

unsafe fn free_entries(ptr: *mut Entry, cap: usize) {
    dealloc(ptr as *mut u8, Layout::array::<Entry>(cap).unwrap())
}

fn grow_capacity(cap: usize) -> usize {
    if cap < 8 {
        8
    } else {
        cap * 2
    }
}

impl Table {
    pub fn new() -> Self {
        Self {
            ptr: null_mut(),
            cap: 0,
            len: 0,
        }
    }

    unsafe fn adjust_capacity(&mut self, new_cap: usize) {
        assert!(
            new_cap > self.cap,
            "new_cap {} must be greater than current capacity {}",
            new_cap,
            self.cap
        );

        // Allocate new entries
        let new_layout = Layout::array::<Entry>(new_cap).unwrap();
        assert!(
            new_layout.size() <= isize::MAX as usize,
            "allocation too large"
        );
        let new_ptr = alloc::alloc(new_layout) as *mut Entry;
        if new_ptr.is_null() {
            panic!("allocation failed");
        }
        // Set new entries to null
        for i in 0..new_cap {
            new_ptr.offset(i as isize).write(Entry {
                key: null_mut(),
                value: Value::Nil,
            });
        }

        // Copy old entries to new entries
        for i in 0..self.cap {
            let entry = self.ptr.offset(i as isize);
            if (*entry).key.is_null() {
                continue;
            }
            let dest = find_entry(new_ptr, new_cap, (*entry).key);
            (*dest).key = (*entry).key;
            (*dest).value = (*entry).value;
        }

        // Free old entries
        free_entries(self.ptr, self.cap);

        // Update table
        self.ptr = new_ptr;
        self.cap = new_cap;
    }

    /// Sets the value of the key in the table. Returns true if the key was
    /// *not* already present in the table.
    ///
    /// Please note that keys are compared using **pointer equality**.
    pub fn set(&mut self, key: &ObjString, value: Value) -> bool {
        if self.len + 1 > (self.cap as f64 * TABLE_MAX_LOAD) as usize {
            unsafe {
                self.adjust_capacity(grow_capacity(self.cap));
            }
        }
        unsafe {
            let mut entry = find_entry(self.ptr, self.cap, key);
            let is_new_key = (*entry).key.is_null();
            (*entry).key = key;
            (*entry).value = value;
            (*entry).value = value;
            if is_new_key {
                self.len += 1;
            }
            return is_new_key;
        }
    }
}

impl Drop for Table {
    fn drop(&mut self) {
        unsafe { free_entries(self.ptr, self.cap) };
    }
}

#[cfg(test)]
mod tests {
    use crate::GC;
    use super::*;

    #[test]
    fn test_simple_set() {
        let mut table = Table::new();
        let mut gc = GC::new();

        let foo_ref = gc.alloc_string("foo".to_string());
        let foo = &(&*foo_ref).unwrap_string();
        assert!(table.set(foo, Value::Nil));
        assert!(!table.set(foo, Value::Nil));
        
        let bar_ref = gc.alloc_string("bar".to_string());
        let bar = &(&*bar_ref).unwrap_string();
        assert!(table.set(bar, Value::Nil));
        assert!(!table.set(bar, Value::Nil));

        let baz_ref = gc.alloc_string("baz".to_string());
        let baz = &(&*baz_ref).unwrap_string();
        assert!(table.set(baz, Value::Nil));
        assert!(!table.set(baz, Value::Nil));
    }

    #[test]
    fn test_setting_256_values() {
        let mut gc = GC::new();
        // We need to use a large number of keys to trigger the capacity
        // adjustment at least a few times.
        let mut table = Table::new();
        for i in 0..256 {
            let key_ref = gc.alloc_string(format!("key_{}", i));
            let key = &(&*key_ref).unwrap_string();
            println!("Setting key {:?} at address {:p}", key, &key);
            assert!(table.set(key, Value::Nil));
            assert!(!table.set(key, Value::Nil));
        }
    }
}
